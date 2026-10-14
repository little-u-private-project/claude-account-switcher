# Shared helpers for claude-instance-start.ps1 / claude-instance-stop.ps1 -
# both dot-source this rather than each keeping its own copy.

$script:InstanceRoot = Join-Path $env:USERPROFILE '.claude-instances'
$script:InstanceStateFile = Join-Path $script:InstanceRoot 'instances.json'

function Get-InstanceState {
    if (-not (Test-Path $script:InstanceStateFile)) { return @() }
    try {
        $raw = Get-Content $script:InstanceStateFile -Raw
        if ([string]::IsNullOrWhiteSpace($raw)) { return @() }
        $j = $raw | ConvertFrom-Json
        if ($null -eq $j) { return @() }
        return @($j)
    } catch { return @() }
}

function Save-InstanceState($list) {
    New-Item -ItemType Directory -Force -Path $script:InstanceRoot | Out-Null
    ConvertTo-Json @($list) -Depth 10 | Set-Content -Path $script:InstanceStateFile -Encoding UTF8
}

# Ground truth for "is this account's isolated instance actually running":
# find a live Claude.exe whose command line references this account's
# --user-data-dir folder. Returns the live PID, or $null if not running.
#
# Deliberately NOT $proc.Path / Process.MainModule (Get-Process's .Path is
# the same underlying MainModule query) - Claude Desktop runs as an
# MSIX/Store app, and MainModule needs process access that its AppContainer
# sandboxing denies across the boundary, so that check throws/returns null
# and silently reports every running instance as not running. This mirrors
# exactly what the GUI's InstanceConfig.IsAccountRunning (C#) does via WMI,
# for the same reason - keep both in sync if this ever changes.
function Test-AccountRunning($instanceDir) {
    if (-not $instanceDir) { return $null }
    try {
        Get-CimInstance Win32_Process -Filter "Name='Claude.exe'" -ErrorAction Stop |
            Where-Object { $_.CommandLine -and $_.CommandLine -like "*$instanceDir*" } |
            Select-Object -First 1 -ExpandProperty ProcessId
    } catch {
        $null
    }
}
