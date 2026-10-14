<#
    Claude Account Switcher - Claude Code (CLI) switcher installer
    (vendored local copy, see
    private-project-vault/01_raw/issues/claude-desktop-account-switch-without-logout.md
    for the security review this copy was installed from)

    Claude Desktop is handled by the separate "Claude Accounts" app (app/, a
    Windows/macOS installer - see README). This script only installs the CLI
    switcher; it runs uninstall.ps1 first, which also removes the retired
    desktop switcher of earlier versions.

    Local install (from this folder, not from GitHub):
      Windows (PowerShell):
        .\install.ps1
      macOS / Linux (needs PowerShell - https://aka.ms/powershell):
        pwsh -c ./install.ps1

    Installs:
      - ~/.claude-tools/claude-code-switch.ps1, claude-code-add.ps1, claude-code-doctor.ps1
      - claude-code-switch / claude-code-add / claude-code-doctor commands
        (PowerShell profiles on Windows, ~/.bashrc / ~/.zshrc elsewhere)
      - Windows: Desktop shortcuts for the switcher and the claude CLI

    No auto-update: claude-switch-update prints a reminder to review the
    upstream diff and re-run this local installer, it never fetches/executes
    remote code on its own.
#>

$RawBase      = 'https://raw.githubusercontent.com/karthiknl0/claude-account-switcher/main'
$SrcDir       = Join-Path $PSScriptRoot 'src'
$ToolsDir     = Join-Path $HOME '.claude-tools'
$CcSwitchPath = Join-Path $ToolsDir 'claude-code-switch.ps1'
$CcAddPath    = Join-Path $ToolsDir 'claude-code-add.ps1'
$CcDoctorPath = Join-Path $ToolsDir 'claude-code-doctor.ps1'
$VersionPath  = Join-Path $ToolsDir '.version'

# -- Platform detection ($IsWindows etc. only exist on PowerShell Core/7+) -----
$PlatformWin = $true; $PlatformMac = $false
if (Test-Path variable:IsWindows) { $PlatformWin = $IsWindows }
if (Test-Path variable:IsMacOS)   { $PlatformMac = $IsMacOS }

$beginMark = '# >>> claude-account-switcher >>>'
$endMark   = '# <<< claude-account-switcher <<<'

Write-Host ""
Write-Host "=== Claude Account Switcher (Claude Code CLI) - installer ===" -ForegroundColor Cyan

# 0. Start clean: the uninstaller removes the previous install and every piece
#    of the retired desktop switcher (one list, kept there); steps 1-3 put back
#    what this version owns.
& (Join-Path $PSScriptRoot 'uninstall.ps1')

# 1. Copy the scripts from this vendored, already-reviewed local copy.
#    ponytail: installs from $SrcDir instead of re-downloading from GitHub on
#    every run, so nothing this installer runs was fetched live/unreviewed.
Write-Host "[1/3] Installing scripts to $ToolsDir ..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $ToolsDir | Out-Null
try {
    Copy-Item "$SrcDir/claude-code-switch.ps1" $CcSwitchPath -Force
    Copy-Item "$SrcDir/claude-code-add.ps1"    $CcAddPath -Force
    Copy-Item "$SrcDir/claude-code-doctor.ps1" $CcDoctorPath -Force
} catch {
    Write-Host "ERROR: could not copy scripts from $SrcDir." -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    return
}
try {
    $ver = (Get-Content (Join-Path $PSScriptRoot 'VERSION') -Raw).Trim()
    Set-Content -Path $VersionPath -Value $ver -Encoding ASCII
    Write-Host "      Installed version $ver" -ForegroundColor DarkGray
} catch {}

if ($PlatformWin) {
    # ---------------------------------------------------------------- Windows --
    Write-Host "[2/3] Adding Claude Code switch commands to your PowerShell profiles..." -ForegroundColor Cyan
    $docs = [Environment]::GetFolderPath('MyDocuments')   # honours OneDrive redirection
    $profiles = @(
        (Join-Path $docs 'WindowsPowerShell\Microsoft.PowerShell_profile.ps1'),
        (Join-Path $docs 'PowerShell\Microsoft.PowerShell_profile.ps1')
    )
    # Marker-delimited block so re-running the installer cleanly REPLACES the old
    # definitions (also drops the retired claude-switch-account / claude-add-account).
    $func = @"
$beginMark
# Claude Code account switcher (managed by claude-account-switcher installer)
function claude-code-switch    { & "$CcSwitchPath" }
function claude-code-add       { & "$CcAddPath" }
function claude-code-doctor    { & "$CcDoctorPath" }
function claude-switch-update {
    # ponytail: no auto-pull. This is a vendored, security-reviewed local copy
    # (private-project/claude-account-switcher) - an update means pulling and
    # re-reviewing the upstream diff yourself, not blindly `irm | iex`-ing
    # whatever is on main right now.
    Write-Host "Auto-update is disabled for this vendored copy." -ForegroundColor Yellow
    Write-Host "To update: review the diff, then re-run the local installer." -ForegroundColor Cyan
    Write-Host "  Compare : $RawBase/VERSION" -ForegroundColor Gray
    Write-Host "  Repo    : https://github.com/karthiknl0/claude-account-switcher" -ForegroundColor Gray
}
$endMark
"@
    foreach ($pf in $profiles) {
        $dir = Split-Path $pf
        if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
        if (-not (Test-Path $pf))  { New-Item -ItemType File -Force -Path $pf | Out-Null }
        $c = Get-Content $pf -Raw -ErrorAction SilentlyContinue
        if ($null -eq $c) { $c = '' }
        $pattern = [Regex]::Escape($beginMark) + '.*?' + [Regex]::Escape($endMark)
        $c = [Regex]::Replace($c, $pattern, '', 'Singleline')
        # Legacy cleanup: drop any old un-delimited block from earlier versions.
        $c = $c -replace '(?s)\r?\n#\s*Claude desktop account switcher[^\r\n]*.*?function\s+claude-add-account\s*\{[^}]*\}', ''
        $c = $c.TrimEnd() + "`r`n`r`n" + $func + "`r`n"
        Set-Content -Path $pf -Value $c -Encoding UTF8
    }

    Write-Host "[3/3] Creating Desktop shortcuts..." -ForegroundColor Cyan
    $desktop = [Environment]::GetFolderPath('Desktop')
    $psExe   = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $ws      = New-Object -ComObject WScript.Shell

    # Claude's MSIX install folder is version-stamped and replaced on every
    # update, so shortcuts get a stable copy of its icon instead.
    $icon = $null
    $pkg  = Get-AppxPackage | Where-Object { $_.PackageFamilyName -eq 'Claude_pzs8sxrjxfjjc' } | Select-Object -First 1
    if ($pkg) {
        $stableIco = Join-Path $ToolsDir 'claude.ico'
        $srcIco    = Join-Path $pkg.InstallLocation 'app\resources\ion-dist\favicon.ico'
        try { if (Test-Path $srcIco) { Copy-Item $srcIco $stableIco -Force } } catch { }
        if (Test-Path $stableIco) { $icon = "$stableIco,0" }
    }

    function New-Shortcut($name, $arguments, $desc) {
        $sc = $ws.CreateShortcut((Join-Path $desktop $name))
        $sc.TargetPath       = $psExe
        $sc.Arguments        = $arguments
        $sc.WorkingDirectory = $HOME
        if ($icon) { $sc.IconLocation = $icon }
        $sc.Description      = $desc
        $sc.WindowStyle      = 1
        $sc.Save()
    }
    New-Shortcut 'Claude Code Switch.lnk' "-ExecutionPolicy Bypass -NoProfile -File `"$CcSwitchPath`"" 'Switch the Claude Code (CLI) account'
    if (Get-Command claude -ErrorAction SilentlyContinue) {
        New-Shortcut 'Claude Code.lnk'          '-NoExit -Command "claude "'           'Launch Claude Code (claude)'
        New-Shortcut 'Claude Code Resume.lnk'   '-NoExit -Command "claude --resume"'   'Resume a Claude Code session (claude --resume)'
        New-Shortcut 'Claude Code Continue.lnk' '-NoExit -Command "claude --continue"' 'Continue the last Claude Code session (claude --continue)'
    } else {
        Write-Host "      (claude CLI not on PATH - skipped Claude Code launch shortcuts)" -ForegroundColor DarkGray
    }

    Write-Host ""
    Write-Host "Done! Claude Code account switcher is installed." -ForegroundColor Green
    Write-Host ""
    Write-Host "Open a NEW PowerShell window so the commands load:" -ForegroundColor Cyan
    Write-Host "  claude-code-add         save each account (after /login in Claude Code)" -ForegroundColor Gray
    Write-Host "  claude-code-switch      switch, then 'claude --resume' to continue work" -ForegroundColor Gray
    Write-Host "  claude-code-doctor      check setup (platform, credentials, accounts)" -ForegroundColor Gray
    Write-Host "  claude-switch-update    how to update" -ForegroundColor Gray
    Write-Host ""
    Write-Host "For Claude Desktop, install the Claude Accounts app (see README)." -ForegroundColor Gray
}
else {
    # ----------------------------------------------------------- macOS / Linux --
    # Shell functions that invoke the .ps1 with `pwsh`, so the user gets plain
    # `claude-code-switch` commands in their normal bash/zsh shell.
    Write-Host "[2/2] Adding Claude Code switch commands to your shell (bash/zsh)..." -ForegroundColor Cyan

    if (-not (Get-Command pwsh -ErrorAction SilentlyContinue)) {
        Write-Host "WARNING: 'pwsh' was not found on PATH." -ForegroundColor Yellow
        Write-Host "Install PowerShell (https://aka.ms/powershell) so the commands can run." -ForegroundColor Yellow
    }

    $block = @"
$beginMark
# Claude Code account switcher (managed by claude-account-switcher installer)
claude-code-switch() { pwsh -NoProfile -File "$CcSwitchPath"; }
claude-code-add()    { pwsh -NoProfile -File "$CcAddPath"; }
claude-code-doctor() { pwsh -NoProfile -File "$CcDoctorPath"; }
claude-switch-update() { pwsh -NoProfile -Command "Write-Host 'Auto-update is disabled for this vendored copy. Review the diff at https://github.com/karthiknl0/claude-account-switcher then re-run the local installer.' -ForegroundColor Yellow"; }
$endMark
"@

    $touched = @()
    foreach ($rc in @((Join-Path $HOME '.bashrc'), (Join-Path $HOME '.zshrc'))) {
        # Only update an rc file that already exists, plus always ensure ~/.bashrc
        # exists on Linux and ~/.zshrc on macOS (the common default shells).
        $isDefault = ($PlatformMac -and $rc.EndsWith('.zshrc')) -or ((-not $PlatformMac) -and $rc.EndsWith('.bashrc'))
        if (-not (Test-Path $rc) -and -not $isDefault) { continue }
        if (-not (Test-Path $rc)) { New-Item -ItemType File -Force -Path $rc | Out-Null }
        $c = Get-Content $rc -Raw -ErrorAction SilentlyContinue
        if ($null -eq $c) { $c = '' }
        $pattern = [Regex]::Escape($beginMark) + '.*?' + [Regex]::Escape($endMark)
        $c = [Regex]::Replace($c, $pattern, '', 'Singleline')
        $c = $c.TrimEnd() + "`n`n" + $block + "`n"
        # Write LF-only, no BOM (shells choke on BOM/CRLF).
        [System.IO.File]::WriteAllText($rc, ($c -replace "`r`n", "`n"), (New-Object System.Text.UTF8Encoding($false)))
        $touched += $rc
    }

    Write-Host ""
    Write-Host "Done! Claude Code account switcher is installed." -ForegroundColor Green
    if ($PlatformMac) {
        Write-Host "Note: macOS stores the Claude Code login in the Keychain. The first time" -ForegroundColor DarkGray
        Write-Host "the switcher reads/writes it, macOS may ask you to Allow access." -ForegroundColor DarkGray
    }
    Write-Host ""
    Write-Host ("Updated: " + ($touched -join ', ')) -ForegroundColor DarkGray
    Write-Host "Run 'source ~/.bashrc' (or ~/.zshrc), or open a new terminal, then:" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  claude-code-add         save each account (after /login in Claude Code)" -ForegroundColor Gray
    Write-Host "  claude-code-switch      switch, then 'claude --resume' to continue work" -ForegroundColor Gray
    Write-Host "  claude-code-doctor      check setup (platform, credentials, accounts)" -ForegroundColor Gray
    Write-Host "  claude-switch-update    how to update" -ForegroundColor Gray
    Write-Host ""
    Write-Host "For Claude Desktop on macOS, install the Claude Accounts app (see README)." -ForegroundColor Gray
}
