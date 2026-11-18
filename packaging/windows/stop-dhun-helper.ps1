# DHUN Local Helper — Stop Script (Windows)
$DataDir = Join-Path $env:LOCALAPPDATA "DHUN\helper"
$PidFile = Join-Path $DataDir "helper.pid"
$stopped = $false

if (Test-Path $PidFile) {
    $savedPid = (Get-Content $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if ($savedPid -and (Get-Process -Id $savedPid.Trim() -ErrorAction SilentlyContinue)) {
        Stop-Process -Id $savedPid.Trim() -Force -ErrorAction SilentlyContinue
        Write-Host "Stopped DHUN Local Helper (PID: $($savedPid.Trim()))." -ForegroundColor Green
        $stopped = $true
    }
    Remove-Item -Force $PidFile -ErrorAction SilentlyContinue
}

if (-not $stopped) {
    $procs = Get-Process -Name "dhun-local-helper" -ErrorAction SilentlyContinue
    if ($procs) {
        $procs | Stop-Process -Force -ErrorAction SilentlyContinue
        Write-Host "Stopped running dhun-local-helper process(es)." -ForegroundColor Green
    } else {
        Write-Host "DHUN Local Helper is not currently running." -ForegroundColor Yellow
    }
}
