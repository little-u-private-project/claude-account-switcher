# Installs seatdeck-cli.exe from the latest GitHub release into
# %LOCALAPPDATA%\Programs\seatdeck-cli and puts it on your user PATH.
#   irm https://raw.githubusercontent.com/little-u-private-project/claude-account-switcher/main/install.ps1 | iex
# Also removes the PowerShell switcher that earlier versions installed.
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repo = if ($env:SEATDECK_REPO) { $env:SEATDECK_REPO } else { 'little-u-private-project/claude-account-switcher' }
$dir = Join-Path $env:LOCALAPPDATA 'Programs\seatdeck-cli'
$exe = Join-Path $dir 'seatdeck-cli.exe'

New-Item -ItemType Directory -Force -Path $dir | Out-Null
Invoke-WebRequest "https://github.com/$repo/releases/latest/download/seatdeck-cli-windows-x64.exe" -OutFile "$exe.part" -UseBasicParsing
Move-Item -Force "$exe.part" $exe

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($userPath -split ';') -notcontains $dir) {
    [Environment]::SetEnvironmentVariable('Path', (($userPath -split ';' | Where-Object { $_ }) + $dir) -join ';', 'User')
    Write-Host "Added $dir to your PATH - open a new terminal to use it."
}

# The old switcher: scripts in ~/.claude-tools, functions in the PowerShell
# profiles, a desktop shortcut. Saved accounts (~/.claude-cc-accounts) stay.
$old = Join-Path $HOME '.claude-tools'
if (Test-Path (Join-Path $old 'claude-code-switch.ps1')) {
    Remove-Item -Recurse -Force $old
    Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Claude Code Switch.lnk')
    $docs = [Environment]::GetFolderPath('MyDocuments')
    foreach ($pf in @("$docs\WindowsPowerShell\Microsoft.PowerShell_profile.ps1", "$docs\PowerShell\Microsoft.PowerShell_profile.ps1")) {
        if (-not (Test-Path $pf)) { continue }
        $text = Get-Content -Raw $pf
        $clean = $text -replace '(?s)\r?\n?# >>> claude-account-switcher >>>.*?# <<< claude-account-switcher <<<\r?\n?', ''
        if ($clean -ne $text) { Set-Content -Path $pf -Value $clean.TrimEnd() -Encoding UTF8 }
    }
    Write-Host 'Removed the old PowerShell switcher (claude-code-switch / -add / -doctor). Saved accounts are kept.'
}

Write-Host "Installed $(& $exe --version) to $dir"
