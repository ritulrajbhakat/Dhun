# DHUN Local Helper + yt-dlp Resolver — Start Script (Windows)
param(
    [int]$Port = 47821,
    [int]$ResolverPort = 8000,
    [string]$ResolverDir = "C:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service"
)

$ErrorActionPreference = "Stop"
$BinDir = Join-Path $env:LOCALAPPDATA "DHUN\bin"
$DataDir = Join-Path $env:LOCALAPPDATA "DHUN\helper"
$ExePath = Join-Path $BinDir "dhun-local-helper.exe"
$PidFile = Join-Path $DataDir "helper.pid"
$ResolverPidFile = Join-Path $DataDir "resolver.pid"
$TunnelPidFile = Join-Path $DataDir "tunnel.pid"
$LogFile = Join-Path $DataDir "helper.log"
$ResolverLog = Join-Path $DataDir "resolver.log"
$TunnelLog = Join-Path $DataDir "tunnel_$(Get-Random).log"

if (-not (Test-Path $ExePath)) {
    $LocalBuild = Join-Path (Get-Location) "target\release\dhun-local-helper.exe"
    if (Test-Path $LocalBuild) {
        $ExePath = $LocalBuild
    } else {
        Write-Error "dhun-local-helper.exe not found. Run install-dhun-helper.ps1 first."
        exit 1
    }
}

$PythonExe = Join-Path $ResolverDir ".venv\Scripts\python.exe"
if (-not (Test-Path $PythonExe)) {
    Write-Error "Resolver virtual environment not found at $PythonExe"
    exit 1
}

$CloudflaredExe = (Get-Command cloudflared -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)
if (-not $CloudflaredExe) {
    $CloudflaredExe = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages", "$env:ProgramFiles", "$env:ProgramFiles(x86)" -Filter "cloudflared.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $CloudflaredExe) {
    Write-Error "cloudflared.exe not found. Install it via: winget install Cloudflare.cloudflared"
    exit 1
}

New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

# Clean up any previously tracked instances
foreach ($pf in @($PidFile, $ResolverPidFile, $TunnelPidFile)) {
    if (Test-Path $pf) {
        $oldPid = Get-Content $pf -ErrorAction SilentlyContinue
        if ($oldPid) { Stop-Process -Id $oldPid -Force -ErrorAction SilentlyContinue }
        Remove-Item $pf -Force -ErrorAction SilentlyContinue
    }
}
Stop-Process -Name "dhun-local-helper", "cloudflared" -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $DataDir "tunnel_*.log") -Force -ErrorAction SilentlyContinue

# 1. Start local FastAPI yt-dlp resolver
$resolverProc = Start-Process -FilePath $PythonExe `
    -ArgumentList "-m", "uvicorn", "app:app", "--host", "127.0.0.1", "--port", "$ResolverPort" `
    -WorkingDirectory $ResolverDir `
    -WindowStyle Hidden `
    -RedirectStandardError $ResolverLog `
    -PassThru
$resolverProc.Id | Set-Content -Path $ResolverPidFile

# 2. Start Cloudflare Quick Tunnel to expose public HTTPS URL
$tunnelProc = Start-Process -FilePath $CloudflaredExe `
    -ArgumentList "tunnel", "--url", "http://127.0.0.1:$ResolverPort", "--logfile", $TunnelLog `
    -WindowStyle Hidden `
    -PassThru
$tunnelProc.Id | Set-Content -Path $TunnelPidFile

Write-Host "Provisioning Cloudflare HTTPS tunnel for yt-dlp resolver..." -ForegroundColor Cyan
$tunnelUrl = $null
for ($i = 0; $i -lt 30; $i++) {
    if (Test-Path $TunnelLog) {
        $match = Select-String -Path $TunnelLog -Pattern "https://[a-z0-9-]+\.trycloudflare\.com" -ErrorAction SilentlyContinue | Select-Object -Last 1
        if ($match) {
            $tunnelUrl = $match.Matches.Value
            break
        }
    }
    Start-Sleep -Milliseconds 500
}

if (-not $tunnelUrl) {
    Write-Error "Timed out waiting for Cloudflare Tunnel URL. Check log: $TunnelLog"
    exit 1
}

# Wait until Windows DNS resolves the newly minted trycloudflare.com hostname
$tunnelHost = ([uri]$tunnelUrl).Host
for ($i = 0; $i -lt 15; $i++) {
    if (Resolve-DnsName $tunnelHost -ErrorAction SilentlyContinue) { break }
    Start-Sleep -Seconds 1
}
Start-Sleep -Seconds 2

# 3. Start DHUN Local Helper wired to the HTTPS resolver URL
$env:DHUN_LOCAL_PORT = "$Port"
$env:DHUN_AUDIO_RESOLVER_URL = "$tunnelUrl/resolve"

$proc = Start-Process -FilePath $ExePath -WindowStyle Hidden -RedirectStandardOutput $LogFile -PassThru
$proc.Id | Set-Content -Path $PidFile

Start-Sleep -Seconds 2
try {
    $health = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/api/local/health" -TimeoutSec 3
    Write-Host "DHUN Local Helper is RUNNING (PID: $($proc.Id), Version: $($health.version))" -ForegroundColor Green
    Write-Host "  Host API:      http://127.0.0.1:$Port"
    Write-Host "  HTTPS Tunnel:  $tunnelUrl/resolve (Resolver PID: $($resolverProc.Id), Tunnel PID: $($tunnelProc.Id))" -ForegroundColor Green
    if ($health.lan_reachable) {
        Write-Host "  LAN Wi-Fi:     http://$($health.detected_lan_ip):$Port (Ready for guest QR codes)" -ForegroundColor Cyan
    } else {
        Write-Host "  LAN Wi-Fi:     Not detected (connect to a Wi-Fi/LAN network for guest QR access)" -ForegroundColor Yellow
    }
} catch {
    Write-Warning "Helper process started (PID: $($proc.Id)) but health check did not respond yet. Check log: $LogFile"
}