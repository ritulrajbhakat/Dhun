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
foreach ($helperScript in @("launch-dhun-helper.vbs", "start-dhun-helper.ps1", "stop-dhun-helper.ps1")) {
    $src = Join-Path $ScriptDir $helperScript
    if (Test-Path $src) {
        Copy-Item -Force $src (Join-Path $BinDir $helperScript)
    }
}

# Register `dhun://` custom URI protocol handler for the current user (HKCU — no admin required)
$ProtoKey = "HKCU:\Software\Classes\dhun"
$ExeInstalled = Join-Path $BinDir "dhun-local-helper.exe"
$VbsInstalled = Join-Path $BinDir "launch-dhun-helper.vbs"

New-Item -Path $ProtoKey -Force | Out-Null
Set-ItemProperty -Path $ProtoKey -Name "(Default)" -Value "URL:DHUN Local Helper Protocol"
Set-ItemProperty -Path $ProtoKey -Name "URL Protocol" -Value ""

New-Item -Path "$ProtoKey\DefaultIcon" -Force | Out-Null
Set-ItemProperty -Path "$ProtoKey\DefaultIcon" -Name "(Default)" -Value "$ExeInstalled,0"

New-Item -Path "$ProtoKey\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "$ProtoKey\shell\open\command" -Name "(Default)" -Value "wscript.exe `"$VbsInstalled`" `"%1`""

Write-Host ""
Write-Host "DHUN Local Helper installed successfully!" -ForegroundColor Green
Write-Host "  Binary:   $ExeInstalled"
Write-Host "  Data Dir: $DataDir"
Write-Host "  Protocol: dhun://start (registered for current user)"
Write-Host ""
Write-Host "To start the helper:" -ForegroundColor Cyan
Write-Host "  & '$(Join-Path $BinDir 'start-dhun-helper.ps1')'"
Write-Host "  (Or click 'Launch Installed Helper' in the DHUN web app)"
Write-Host ""
Write-Host "NOTE FOR LAN GUEST SMARTPHONES (Windows Firewall):" -ForegroundColor Yellow
Write-Host "  Allow TCP port 47821 on Private networks only so guests on the same Wi-Fi can scan your QR code:"
Write-Host "  New-NetFirewallRule -DisplayName 'DHUN Local Helper (Private LAN)' -Direction Inbound -Protocol TCP -LocalPort 47821 -Profile Private -Action Allow"
