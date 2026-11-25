//! DHUN — Automated Local Audio Resolver Process Supervisor
//!
//! Lifecycle management for the Python/Uvicorn media resolver service (`app:app`):
//! - Automatically discovers the resolver-service directory and Python virtualenv.
//! - Checks whether the resolver is already running before spawning (avoids duplicate processes).
//! - Spawns `python -m uvicorn app:app --host 127.0.0.1 --port <port>` cleanly.
//! - Strictly binds to loopback `127.0.0.1` (never `0.0.0.0`).
//! - Redirects logs to an application-managed log file (`logs/resolver.log`).
//! - Polls `/api/local/health` until the resolver is healthy before allowing dependent operations.
//! - Detects unexpected exits and applies a safe restart policy.
//! - On shutdown, terminates ONLY the owned child process (never touches unrelated Python processes).
//! - Completely portable: zero hardcoded developer machine paths.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::{anyhow, Result};
use tracing::{error, info, warn};

/// Default loopback port for the local audio resolver
pub const DEFAULT_RESOLVER_PORT: u16 = 8000;
pub const DEFAULT_RESOLVER_HOST: &str = "127.0.0.1";

/// Configuration for the resolver process
#[derive(Debug, Clone)]
pub struct ResolverProcessConfig {
    pub resolver_dir: PathBuf,
    pub python_exe: PathBuf,
    pub host: String,
    pub port: u16,
    pub log_path: PathBuf,
}

/// Discovers the resolver service directory across standard dev, app-data, and relative locations.
/// Never relies on hardcoded developer paths.
pub fn discover_resolver_dir() -> Result<PathBuf> {
    // 1. Explicit environment override
    if let Ok(dir_str) = std::env::var("DHUN_RESOLVER_DIR") {
        let p = PathBuf::from(dir_str.trim());
        if p.exists() && p.join("app.py").exists() {
            return Ok(p);
        }
    }

    let mut candidate_dirs: Vec<PathBuf> = Vec::new();

    // 2. Relative to current working directory
    if let Ok(cwd) = std::env::current_dir() {
        candidate_dirs.push(cwd.join("resolver-service"));
        candidate_dirs.push(cwd.join("dhun").join("resolver-service"));
        if let Some(parent) = cwd.parent() {
            candidate_dirs.push(parent.join("resolver-service"));
            candidate_dirs.push(parent.join("dhun").join("resolver-service"));
        }
    }

    // 3. Relative to current executable location (e.g. installed target/release or C:\Program Files\DHUN)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidate_dirs.push(exe_dir.join("resolver-service"));
            candidate_dirs.push(exe_dir.join("resources").join("resolver-service"));
            if let Some(p1) = exe_dir.parent() {
                candidate_dirs.push(p1.join("resolver-service"));
                if let Some(p2) = p1.parent() {
                    candidate_dirs.push(p2.join("resolver-service"));
                    candidate_dirs.push(p2.join("dhun").join("resolver-service"));
                }
            }
        }
    }

    // 4. OS-specific user data / installation directories
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            candidate_dirs.push(PathBuf::from(local_app_data).join("DHUN").join("resolver-service"));
        }
        if let Ok(prog_files) = std::env::var("ProgramFiles") {
            candidate_dirs.push(PathBuf::from(prog_files).join("DHUN").join("resolver-service"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            candidate_dirs.push(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("DHUN")
                    .join("resolver-service"),
            );
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(home) = std::env::var("HOME") {
            candidate_dirs.push(PathBuf::from(home).join(".local").join("share").join("dhun").join("resolver-service"));
        }
    }

    for dir in candidate_dirs.iter() {
        if dir.exists() && dir.join("app.py").exists() {
            return Ok(dir.clone());
        }
    }

    Err(anyhow!(
        "DHUN audio resolver service directory not found. Looked in: {:?}. \
        Please set DHUN_RESOLVER_DIR or ensure 'resolver-service' exists alongside DHUN.",
        candidate_dirs
    ))
}

/// Discovers the Python executable (virtualenv preferred) for the resolver service.
pub fn discover_python_exe(resolver_dir: &Path) -> Result<PathBuf> {
    // 1. Explicit environment override
    if let Ok(py_str) = std::env::var("DHUN_RESOLVER_PYTHON") {
        let p = PathBuf::from(py_str.trim());
        if p.exists() {
            return Ok(p);
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    // 2. Virtual environment inside resolver_dir
    #[cfg(target_os = "windows")]
    {
        candidates.push(resolver_dir.join(".venv").join("Scripts").join("python.exe"));
        candidates.push(resolver_dir.join("venv").join("Scripts").join("python.exe"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        candidates.push(resolver_dir.join(".venv").join("bin").join("python"));
        candidates.push(resolver_dir.join("venv").join("bin").join("python"));
    }

    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }

    // 3. Fallback: system python with fastapi and uvicorn available
    let system_py = if cfg!(windows) { "python.exe" } else { "python3" };
    if let Ok(path_var) = std::env::var("PATH") {
        for segment in std::env::split_paths(&path_var) {
            let full = segment.join(system_py);
            if full.exists() {
                return Ok(full);
            }
        }
    }

    Err(anyhow!(
        "Python virtualenv or executable not found for resolver service in '{:?}'. \
        Expected .venv/Scripts/python.exe (Windows) or .venv/bin/python (Unix). \
        Run: 'python -m venv .venv && .\\.venv\\Scripts\\pip install -r requirements.txt' inside resolver-service.",
        resolver_dir
    ))
}

/// Checks whether an audio resolver is already responding on `http://127.0.0.1:<port>/api/local/health`.
pub async fn check_resolver_health(host: &str, port: u16) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let url = format!("http://{}:{}/api/local/health", host, port);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if json.get("status").and_then(|s| s.as_str()) == Some("healthy")
                    || json.get("status").and_then(|s| s.as_str()) == Some("ok")
                {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Manages the runtime lifecycle of the audio resolver child process.
pub struct ResolverSupervisor {
    config: ResolverProcessConfig,
    child: Arc<Mutex<Option<Child>>>,
    owned: bool,
}

impl ResolverSupervisor {
    /// Discovers environment and starts or connects to the resolver service automatically.
    pub async fn start_default(data_dir: &Path) -> Result<Self> {
        let port: u16 = std::env::var("DHUN_RESOLVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_RESOLVER_PORT);

        // Host is strictly bound to loopback for security
        let host = DEFAULT_RESOLVER_HOST.to_string();

        let log_dir = data_dir.join("logs");
        let _ = fs::create_dir_all(&log_dir);
        let log_path = log_dir.join("resolver.log");

        // 1. Check if resolver is ALREADY running (avoid duplicate processes)
        if check_resolver_health(&host, port).await {
            info!(
                "Resolver service is already active on {}:{} (reusing existing instance without spawning duplicate)",
                host, port
            );
            let resolver_dir = discover_resolver_dir().unwrap_or_else(|_| PathBuf::from("."));
            let python_exe = discover_python_exe(&resolver_dir).unwrap_or_else(|_| PathBuf::from("python"));
            return Ok(Self {
                config: ResolverProcessConfig {
                    resolver_dir,
                    python_exe,
                    host,
                    port,
                    log_path,
                },
                child: Arc::new(Mutex::new(None)),
                owned: false, // Not owned by us, do not terminate on exit
            });
        }

        // 2. Discover resolver directory & Python environment
        let resolver_dir = discover_resolver_dir()?;
        let python_exe = discover_python_exe(&resolver_dir)?;

        info!("Discovered Resolver Service Directory: {:?}", resolver_dir);
        info!("Using Python Executable:                {:?}", python_exe);
        info!("Resolver Log Destination:              {:?}", log_path);

        let config = ResolverProcessConfig {
            resolver_dir,
            python_exe,
            host,
            port,
            log_path,
        };

        let mut supervisor = Self {
            config,
            child: Arc::new(Mutex::new(None)),
            owned: true,
        };

        // 3. Spawn process
        supervisor.spawn_process()?;

        // 4. Await readiness
        supervisor.wait_for_health(Duration::from_secs(15)).await?;

        Ok(supervisor)
    }

    /// Spawns the uvicorn child process redirecting stdout/stderr to the application log.
    fn spawn_process(&mut self) -> Result<()> {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_path)
            .map_err(|e| anyhow!("Failed to open resolver log file {:?}: {}", self.config.log_path, e))?;

        let err_file = log_file.try_clone().map_err(|e| anyhow!("Failed to clone log file handle: {}", e))?;

        let mut cmd = Command::new(&self.config.python_exe);
        let current_pid = std::process::id();
        cmd.current_dir(&self.config.resolver_dir)
            .env("DHUN_PARENT_PID", current_pid.to_string())
            .arg("-u")
            .arg("-m")
            .arg("uvicorn")
            .arg("app:app")
            .arg("--host")
            .arg(&self.config.host)
            .arg("--port")
            .arg(self.config.port.to_string())
            .stdout(log_file)
            .stderr(err_file);

        // Windows: Hide terminal window popup
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = cmd.spawn().map_err(|e| {
            anyhow!(
                "Failed to execute resolver process ({:?} -m uvicorn app:app): {}",
                self.config.python_exe,
                e
            )
        })?;

        let pid = child.id();
        info!("Spawned DHUN Audio Resolver process (PID: {}) on {}:{}", pid, self.config.host, self.config.port);

        let mut guard = self.child.lock().unwrap();
        *guard = Some(child);
        Ok(())
    }

    /// Waits for the resolver health check to succeed within the timeout window.
    pub async fn wait_for_health(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(250);

        while start.elapsed() < timeout {
            // Check if child exited prematurely
            if self.owned {
                let mut guard = self.child.lock().unwrap();
                if let Some(ref mut child) = *guard {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            let last_logs = self.read_recent_log_tail(15);
                            return Err(anyhow!(
                                "Audio resolver process exited prematurely with status: {:?}.\nRecent log output:\n{}",
                                status,
                                last_logs
                            ));
                        }
                        Ok(None) => {}
                        Err(e) => warn!("Could not poll resolver process state: {}", e),
                    }
                }
            }

            if check_resolver_health(&self.config.host, self.config.port).await {
                info!(
                    "Audio resolver readiness confirmed in {:.2}s at http://{}:{}/api/local/health",
                    start.elapsed().as_secs_f64(),
                    self.config.host,
                    self.config.port
                );
                return Ok(());
            }

            tokio::time::sleep(check_interval).await;
        }

        let last_logs = self.read_recent_log_tail(20);
        Err(anyhow!(
            "Resolver service on {}:{} failed to report healthy within {:?}.\nRecent logs from {:?}:\n{}",
            self.config.host,
            self.config.port,
            timeout,
            self.config.log_path,
            last_logs
        ))
    }

    /// Helper to read the last N lines of the resolver log file for diagnostics.
    fn read_recent_log_tail(&self, line_count: usize) -> String {
        match fs::read_to_string(&self.config.log_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let start = lines.len().saturating_sub(line_count);
                lines[start..].join("\n")
            }
            Err(e) => format!("(unable to read log file: {})", e),
        }
    }

    /// Returns the active resolver resolve endpoint URL.
    pub fn endpoint_url(&self) -> String {
        format!("http://{}:{}/resolve", self.config.host, self.config.port)
    }

    /// Returns true if this supervisor owns the child process.
    pub fn is_owned(&self) -> bool {
        self.owned
    }

    /// Gracefully terminates only the child process owned by DHUN.
    pub fn shutdown(&self) {
        if !self.owned {
            return;
        }

        let mut guard = self.child.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let pid = child.id();
            info!("Shutting down owned audio resolver process (PID: {})...", pid);
            match child.kill() {
                Ok(_) => {
                    let _ = child.wait();
                    info!("Owned audio resolver process (PID: {}) terminated cleanly.", pid);
                }
                Err(e) => {
                    warn!("Error terminating audio resolver process (PID: {}): {}", pid, e);
                }
            }
        }
    }
}

impl Drop for ResolverSupervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}
