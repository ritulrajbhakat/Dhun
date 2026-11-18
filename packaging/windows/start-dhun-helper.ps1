# DHUN Local Helper — Start Script (Windows)
param(
    [int]$Port = 47821
)

$ErrorActionPreference = "Stop"
$BinDir = Join-Path $env:LOCALAPPDATA "DHUN\bin"
$DataDir = Join-Path $env:LOCALAPPDATA "DHUN\helper"
$ExePath = Join-Path $BinDir "dhun-local-helper.exe"
$PidFile = Join-Path $DataDir "helper.pid"
$LogFile = Join-Path $DataDir "helper.log"

if (-not (Test-Path $ExePath)) {
    $LocalBuild = Join-Path (Get-Location) "target\release\dhun-local-helper.exe"
    if (Test-Path $LocalBuild) {
        $ExePath = $LocalBuild
    } else {
        Write-Error "dhun-local-helper.exe not found. Run install-dhun-helper.ps1 first."
        exit 1
    }
}

New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

$env:DHUN_LOCAL_PORT = "$Port"
$proc = Start-Process -FilePath $ExePath -WindowStyle Hidden -RedirectStandardOutput $LogFile -PassThru
$proc.Id | Set-Content -Path $PidFile

Start-Sleep -Seconds 2
try {
    $health = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/api/local/health" -TimeoutSec 3
    Write-Host "DHUN Local Helper is RUNNING (PID: $($proc.Id), Version: $($health.version))" -ForegroundColor Green
    Write-Host "  Host API:    http://127.0.0.1:$Port"
    if ($health.lan_reachable) {
        Write-Host "  LAN Wi-Fi:   http://$($health.detected_lan_ip):$Port (Ready for guest QR codes)" -ForegroundColor Cyan
    } else {
        Write-Host "  LAN Wi-Fi:   Not detected (connect to a Wi-Fi/LAN network for guest QR access)" -ForegroundColor Yellow
    }
} catch {
    Write-Warning "Helper process started (PID: $($proc.Id)) but health check did not respond yet. Check log: $LogFile"
}
