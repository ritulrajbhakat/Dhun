# DHUN Local Helper — Windows Installation Script
# Installs dhun-local-helper.exe into %LOCALAPPDATA%\DHUN\bin with start/stop helpers.

param(
    [string]$BinaryPath = ".\dhun-local-helper.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $BinaryPath)) {
    Write-Error "Could not find dhun-local-helper.exe at '$BinaryPath'. Build with 'cargo build --release --bin dhun-local-helper' or download the Windows release artifact."
    exit 1
}

$InstallRoot = Join-Path $env:LOCALAPPDATA "DHUN"
$BinDir = Join-Path $InstallRoot "bin"
$DataDir = Join-Path $InstallRoot "helper"

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

Copy-Item -Force $BinaryPath (Join-Path $BinDir "dhun-local-helper.exe")

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
foreach ($helperScript in @("start-dhun-helper.ps1", "stop-dhun-helper.ps1")) {
    $src = Join-Path $ScriptDir $helperScript
    if (Test-Path $src) {
        Copy-Item -Force $src (Join-Path $BinDir $helperScript)
    }
}

Write-Host ""
Write-Host "DHUN Local Helper installed successfully!" -ForegroundColor Green
Write-Host "  Binary:   $(Join-Path $BinDir 'dhun-local-helper.exe')"
Write-Host "  Data Dir: $DataDir"
Write-Host ""
Write-Host "To start the helper:" -ForegroundColor Cyan
Write-Host "  & '$(Join-Path $BinDir 'start-dhun-helper.ps1')'"
Write-Host ""
Write-Host "NOTE FOR LAN GUEST SMARTPHONES (Windows Firewall):" -ForegroundColor Yellow
Write-Host "  Allow TCP port 47821 on Private networks only so guests on the same Wi-Fi can scan your QR code:"
Write-Host "  New-NetFirewallRule -DisplayName 'DHUN Local Helper (Private LAN)' -Direction Inbound -Protocol TCP -LocalPort 47821 -Profile Private -Action Allow"
