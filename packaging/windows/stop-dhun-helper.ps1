# DHUN Local Helper + yt-dlp Resolver — Stop Script (Windows)
$DataDir = Join-Path $env:LOCALAPPDATA "DHUN\helper"
$Services = @(
    @{ Name = "DHUN Local Helper"; PidFile = (Join-Path $DataDir "helper.pid");   ProcName = "dhun-local-helper" },
    @{ Name = "Cloudflare Tunnel"; PidFile = (Join-Path $DataDir "tunnel.pid");   ProcName = "cloudflared" },
    @{ Name = "yt-dlp Resolver";   PidFile = (Join-Path $DataDir "resolver.pid"); ProcName = $null }
)

foreach ($svc in $Services) {
    $stopped = $false

    if (Test-Path $svc.PidFile) {
        $savedPid = (Get-Content $svc.PidFile -ErrorAction SilentlyContinue | Select-Object -First 1).Trim()
        if ($savedPid -and (Get-Process -Id $savedPid -ErrorAction SilentlyContinue)) {
            Stop-Process -Id $savedPid -Force -ErrorAction SilentlyContinue
            Write-Host "Stopped $($svc.Name) (PID: $savedPid)." -ForegroundColor Green
            $stopped = $true
        }
        Remove-Item -Force $svc.PidFile -ErrorAction SilentlyContinue
    }

    if (-not $stopped -and $svc.ProcName) {
        $procs = Get-Process -Name $svc.ProcName -ErrorAction SilentlyContinue
        if ($procs) {
            $procs | Stop-Process -Force -ErrorAction SilentlyContinue
            Write-Host "Stopped running $($svc.Name) process(es)." -ForegroundColor Green
            $stopped = $true
        }
    }

    if (-not $stopped) {
        Write-Host "$($svc.Name) is not currently running." -ForegroundColor Yellow
    }
}

# Clean up temporary tunnel log files
Remove-Item (Join-Path $DataDir "tunnel_*.log") -Force -ErrorAction SilentlyContinue