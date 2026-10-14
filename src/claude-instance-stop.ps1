# Stop the concurrently-running Claude Desktop instance for -Email, and only
# that one - never kill-by-name (that's claude-switch.ps1's job, correct only
# for its own single-instance model). Finds the actual live process by its
# --user-data-dir (ground truth) rather than trusting a possibly-stale
# recorded pid.

param(
    [Parameter(Mandatory)][string]$Email
)

. (Join-Path $PSScriptRoot 'claude-instances-common.ps1')

$state = Get-InstanceState
$entry = $state | Where-Object { $_.email -eq $Email } | Select-Object -First 1

if (-not $entry) {
    Write-Host "$Email is not a saved account." -ForegroundColor Yellow
    exit 0
}

$livePid = Test-AccountRunning $entry.userDataDir
if ($livePid) {
    Stop-Process -Id $livePid -Force
    Write-Host "Stopped $Email (PID $livePid)." -ForegroundColor Green
} else {
    Write-Host "$Email was not running." -ForegroundColor DarkGray
}

$state = $state | ForEach-Object {
    if ($_.email -eq $Email) { $_.pid = $null }
    $_
}
Save-InstanceState $state
