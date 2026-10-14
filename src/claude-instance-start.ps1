# Launch a Claude Desktop instance for -Email concurrently with any other
# already-running instances, each isolated via its own --user-data-dir.
# Idempotent: if that account is already running, does nothing.
#
# Deliberately separate from claude-switch.ps1's model: that script swaps a
# single shared profile/ folder in place, which would corrupt a long-running
# concurrent instance if the two ever touched the same folder. This script
# seeds and launches its own instance/ folder instead, and never touches
# profile/ after the initial one-time seed copy.

param(
    [Parameter(Mandatory)][string]$Email
)

. (Join-Path $PSScriptRoot 'claude-instances-common.ps1')

$store        = Join-Path $env:USERPROFILE '.claude-accounts'

function Get-ClaudeExePath {
    $pkg = Get-AppxPackage | Where-Object { $_.PackageFamilyName -eq 'Claude_pzs8sxrjxfjjc' } | Select-Object -First 1
    if ($pkg) {
        $cand = Join-Path $pkg.InstallLocation 'app\Claude.exe'
        if (Test-Path $cand) { return $cand }
    }
    return $null
}

function Copy-ClaudeProfile($source, $destination) {
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    robocopy $source $destination /MIR /COPY:DAT /DCOPY:DAT /XJ /R:1 /W:1 /NFL /NDL /NJH /NJS | Out-Null
    if ($LASTEXITCODE -gt 7) { return $false }
    Get-ChildItem $destination -Force -Filter 'Singleton*' -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
    return $true
}

$safe        = ($Email -replace '[^\w.@+-]', '_')
$accountDir  = Join-Path $store $safe
$profileDir  = Join-Path $accountDir 'profile'
$instanceDir = Join-Path $accountDir 'instance'

if (-not (Test-Path $profileDir)) {
    Write-Host "No saved account for $Email - run 'claude-add-account' first." -ForegroundColor Red
    exit 1
}

$state = Get-InstanceState
$entry = $state | Where-Object { $_.email -eq $Email } | Select-Object -First 1

$livePid = Test-AccountRunning $instanceDir
if ($livePid) {
    Write-Host "$Email is already running (PID $livePid)." -ForegroundColor Yellow
    exit 0
}

if (-not (Test-Path $instanceDir)) {
    Write-Host "Seeding isolated instance folder for $Email..." -ForegroundColor Cyan
    if (-not (Copy-ClaudeProfile $profileDir $instanceDir)) {
        Write-Host "Could not seed the instance folder. No process started." -ForegroundColor Red
        exit 1
    }
}

$exe = Get-ClaudeExePath
if (-not $exe) {
    Write-Host "Could not find Claude Desktop's install location (MSIX package not found)." -ForegroundColor Red
    exit 1
}

Write-Host "Launching $Email ..." -ForegroundColor Cyan
$proc = Start-Process -FilePath $exe -ArgumentList "--user-data-dir=`"$instanceDir`"" -PassThru

$state = @($state | Where-Object { $_.email -ne $Email })
$state += [pscustomobject]@{
    email       = $Email
    userDataDir = $instanceDir
    hotkey      = $entry.hotkey
    pid         = $proc.Id
}
Save-InstanceState $state

Write-Host "Started $Email (PID $($proc.Id))." -ForegroundColor Green
