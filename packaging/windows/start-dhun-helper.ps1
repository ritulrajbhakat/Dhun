# DHUN Local Helper — Portable Start Script (Windows)
# Zero Cloudflare Tunnel, zero hardcoded developer paths.
param(
    [int]$Port = 47821,
    [string]$ResolverUrl = $env:DHUN_AUDIO_RESOLVER_URL,
    [string]$ResolverKey = $env:DHUN_AUDIO_RESOLVER_KEY,
    [switch]$AllowLoopbackResolver
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

# Cleanly stop any previously running helper instance
if (Test-Path $PidFile) {
    $oldPid = (Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if ($oldPid) { Stop-Process -Id $oldPid.Trim() -Force -ErrorAction SilentlyContinue }
    Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
}
Stop-Process -Name "dhun-local-helper" -Force -ErrorAction SilentlyContinue

$env:DHUN_LOCAL_PORT = "$Port"
if (-not $ResolverUrl) {
    foreach ($candPort in @(8000, 8080)) {
        try {
            $probe = Invoke-RestMethod -Uri "http://127.0.0.1:$candPort/api/local/health" -TimeoutSec 1 -ErrorAction SilentlyContinue
            if ($probe -and ($probe.status -eq "healthy" -or $probe.status -eq "ok")) {
                $ResolverUrl = "http://127.0.0.1:$candPort/resolve"
                Write-Host "  Auto-detected active local audio resolver at $ResolverUrl" -ForegroundColor Cyan
                break
            }
        } catch {}
    }
}
if ($ResolverUrl) {
    $env:DHUN_AUDIO_RESOLVER_URL = $ResolverUrl
    if ($AllowLoopbackResolver -or $ResolverUrl -match '^https?://(127\.0\.0\.1|localhost)(:\d+)?/') {
        $env:DHUN_ALLOW_LOOPBACK_RESOLVER = "1"
    }
}
if ($ResolverKey) {
    $env:DHUN_AUDIO_RESOLVER_KEY = $ResolverKey
}

$proc = Start-Process -FilePath $ExePath -WindowStyle Hidden -RedirectStandardOutput $LogFile -PassThru
$proc.Id | Set-Content -Path $PidFile

Start-Sleep -Seconds 2
try {
    $health = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/api/local/health" -TimeoutSec 3
    Write-Host "DHUN Local Helper is RUNNING (PID: $($proc.Id), Version: $($health.version))" -ForegroundColor Green
    Write-Host "  Host API:    http://127.0.0.1:$Port"
    Write-Host "  Resolver:    $(if ($health.audio_resolver_configured) { $env:DHUN_AUDIO_RESOLVER_URL } else { 'Not configured (offline/metadata mode)' })"
    if ($health.lan_reachable) {
        Write-Host "  LAN Wi-Fi:   http://$($health.detected_lan_ip):$Port (Ready for guest QR codes)" -ForegroundColor Cyan
    } else {
        Write-Host "  LAN Wi-Fi:   Not detected (connect to a Wi-Fi/LAN network for guest QR access)" -ForegroundColor Yellow
    }
} catch {
    Write-Warning "Helper process started (PID: $($proc.Id)) but health check did not respond yet. Check log: $LogFile"
}
