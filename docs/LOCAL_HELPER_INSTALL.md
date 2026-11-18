# Installing and Operating the Local DHUN Helper Service

The **Local DHUN Helper (`dhun-local-helper`)** runs on the host's laptop while a party is active. It hosts the private LAN party room, serves guest QR requests over the local Wi-Fi, manages the 10–15 track temporary audio cache, performs local DSP audio analysis, and coordinates automated DJ transitions.

---

## 1. Storage & Configuration Directories

By default, `dhun-local-helper` stores its local SQLite database and temporary rolling audio cache in safe OS-native user directories (never inside arbitrary working directories):

| Platform | Default Storage Path |
|---|---|
| **Windows x86_64** | `%LOCALAPPDATA%\DHUN\helper\` (`dhun_local_helper.db` & `temp_cache\`) |
| **Linux x86_64** | `$XDG_DATA_HOME/dhun/helper/` or `~/.local/share/dhun/helper/` |
| **macOS** | `~/Library/Application Support/DHUN/helper/` |

Optional environment variable overrides:
- `DHUN_LOCAL_PORT` (default `47821`)
- `DHUN_LOCAL_DB_PATH` (explicit SQLite path)
- `DHUN_CACHE_DIR` (explicit temporary audio cache directory)
- `DHUN_ALLOWED_ORIGINS` (comma-separated public Vercel origins if using a custom domain; `*.vercel.app` and `localhost` are already allowed)
- `DHUN_GLOBAL_CATALOGUE_URL` & `DHUN_CATALOGUE_SERVICE_KEY` (optional engine-only metadata sync; omit for 100% offline operation)
- `DHUN_AUDIO_RESOLVER_URL` & optional `DHUN_AUDIO_RESOLVER_KEY` (optional host-configured HTTPS media resolver endpoint that receives `POST` JSON `{ source_url, provider_video_id, title, artist, version_tag }` and returns either JSON `{ direct_https_audio_url, license_or_authorization_note, expected_sha256 }` or direct `audio/*` bytes to cache and analyze locally)

---

## 2. Windows x86_64 Installation & Startup (Verified Build Target)

### Option A — One-Click Windows Installer (Recommended)
1. Download **`DHUN-Local-Helper-Setup.exe`** from the **`DHUN-Local-Helper-Setup-Windows-x86_64`** GitHub Actions artifact (or build locally with Inno Setup 6: `iscc packaging/windows/dhun-helper-setup.iss`).
2. Run **`DHUN-Local-Helper-Setup.exe`**. In one guided step it:
   - Installs `dhun-local-helper.exe` into `%LOCALAPPDATA%\DHUN\bin`.
   - Registers the **`dhun://`** custom protocol handler (`HKCU\Software\Classes\dhun`) so clicking **"Launch Installed Helper"** in the DHUN website starts the helper automatically with one browser confirmation (just like Zoom or Spotify).
   - Optionally configures the Private-Wi-Fi-only Windows Firewall rule for TCP port `47821` (and removes it automatically on uninstall).
   - Optionally enables start-on-Windows-sign-in and starts the helper right away.

### Option B — Manual PowerShell Install (Developer / From Source)
1. Download the `dhun-local-helper-windows-x86_64` artifact from GitHub Actions (or build locally with `cargo build --release --manifest-path src-tauri/Cargo.toml --bin dhun-local-helper`).
2. Run the installation script (also registers the `dhun://start` protocol handler for your user):
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\packaging\windows\install-dhun-helper.ps1 -BinaryPath .\target\release\dhun-local-helper.exe
   ```
3. Allow local Wi-Fi guest smartphones through Windows Firewall on **Private** networks only (Administrator Command Prompt or PowerShell):
   ```cmd
   netsh advfirewall firewall add rule name="DHUN Local Helper (Private LAN)" dir=in action=allow protocol=TCP localport=47821 profile=private
   ```
4. **Start / Stop the Helper:**
   ```powershell
   powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\DHUN\bin\start-dhun-helper.ps1"
   powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\DHUN\bin\stop-dhun-helper.ps1"
   ```

---

## 3. Linux x86_64 / macOS Startup

1. Build `dhun-local-helper`:
   ```bash
   cargo build --release --manifest-path src-tauri/Cargo.toml --bin dhun-local-helper
   ```
   *(Linux requires `libasound2-dev` and `pkg-config` at build time.)*
2. Start / stop using the bundled scripts:
   ```bash
   DHUN_HELPER_BIN=./target/release/dhun-local-helper ./packaging/unix/start-dhun-helper.sh
   ./packaging/unix/stop-dhun-helper.sh
   ```

---

## 4. Verification, Updates, and Crash Recovery

- **Health & Diagnostic Page**: Visit `http://127.0.0.1:47821/` in your host browser to verify the helper is running and see the detected private LAN Wi-Fi IP.
- **Updating**: Stop the helper (`stop-dhun-helper.ps1` / `.sh`), replace the binary by re-running the install script, and start it again. SQLite schema migrations apply automatically on startup without losing reusable cached metadata.
- **Crash / Power-Loss Recovery**: If the host laptop shuts down unexpectedly before explicitly closing a party, the next launch of `dhun-local-helper` automatically reconciles stale cache locks, deletes any temporary audio files older than their 60-minute expiry fallback, and recovers interrupted metadata-only sync outbox items cleanly.
