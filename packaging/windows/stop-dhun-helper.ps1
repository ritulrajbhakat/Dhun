# DHUN Local Helper — Stop Script (Windows)
$PidFile = Join-Path $env:LOCALAPPDATA "DHUN\helper\helper.pid"

if (Test-Path $PidFile) {
    $savedPid = Get-Content $PidFile -ErrorAction SilentlyContinue
    if ($savedPid) {
        Stop-Process -Id $savedPid -ErrorAction SilentlyContinue
        Write-Host "Stopped DHUN Local Helper (PID: $savedPid)." -ForegroundColor Green
    }
    Remove-Item -Force $PidFile -ErrorAction SilentlyContinue
} else {
    $procs = Get-Process -Name "dhun-local-helper" -ErrorAction SilentlyContinue
    if ($procs) {
        $procs | Stop-Process -ErrorAction SilentlyContinue
        Write-Host "Stopped running dhun-local-helper process(es)." -ForegroundColor Green
    } else {
        Write-Host "DHUN Local Helper is not currently running." -ForegroundColor Yellow
    }
}
