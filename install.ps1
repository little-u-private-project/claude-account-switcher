<#
    Claude Account Switcher - installer (vendored local copy, see
    private-project-vault/01_raw/issues/claude-desktop-account-switch-without-logout.md
    for the security review this copy was installed from)

    Local install (from this folder, not from GitHub):
      Windows (PowerShell):
        .\install.ps1
      macOS / Linux (needs PowerShell - https://aka.ms/powershell):
        pwsh -c ./install.ps1

    No auto-update: claude-switch-update prints a reminder to review the
    upstream diff and re-run this local installer, it never fetches/executes
    remote code on its own.

    Installs (Windows):
      - ~/.claude-tools/*.ps1 (desktop + Claude Code switchers)
      - claude-switch-account / claude-add-account / claude-code-switch /
        claude-code-add commands in your PowerShell profiles (5.1 and 7)
      - Desktop shortcuts

    Installs (macOS / Linux):
      - ~/.claude-tools/claude-code-switch.ps1, claude-code-add.ps1
      - claude-code-switch / claude-code-add / claude-switch-update functions in
        your shell (~/.bashrc and ~/.zshrc), each invoking `pwsh`
      (The desktop-app switcher is Windows-only and is skipped.)

    No npm required - it works directly on Claude's credentials.
#>

$RawBase     = 'https://raw.githubusercontent.com/karthiknl0/claude-account-switcher/main'
$SrcDir      = Join-Path $PSScriptRoot 'src'
$ToolsDir    = Join-Path $HOME '.claude-tools'
$SwitchPath  = Join-Path $ToolsDir 'claude-switch.ps1'
$AddPath     = Join-Path $ToolsDir 'claude-add.ps1'
$CcSwitchPath= Join-Path $ToolsDir 'claude-code-switch.ps1'
$CcAddPath   = Join-Path $ToolsDir 'claude-code-add.ps1'
$CcDoctorPath= Join-Path $ToolsDir 'claude-code-doctor.ps1'
$InstStartPath = Join-Path $ToolsDir 'claude-instance-start.ps1'
$InstStopPath  = Join-Path $ToolsDir 'claude-instance-stop.ps1'
$InstCommonPath= Join-Path $ToolsDir 'claude-instances-common.ps1'
$VersionPath = Join-Path $ToolsDir '.version'
$GuiSrcDir   = Join-Path $PSScriptRoot 'gui\ClaudeAccountTray'
$GuiOutDir   = Join-Path $env:LOCALAPPDATA 'ClaudeAccountTray'

# -- Platform detection ($IsWindows etc. only exist on PowerShell Core/7+) -----
$PlatformWin = $true; $PlatformMac = $false
if (Test-Path variable:IsWindows) { $PlatformWin = $IsWindows }
if (Test-Path variable:IsMacOS)   { $PlatformMac = $IsMacOS }

Write-Host ""
Write-Host "=== Claude Account Switcher - installer ===" -ForegroundColor Cyan

# 1. Copy the scripts from this vendored, already-reviewed local copy (CLI
#    switcher works on every platform; the desktop switcher is Windows-only).
#    ponytail: installs from $SrcDir instead of re-downloading from GitHub on
#    every run, so nothing this installer runs was fetched live/unreviewed.
Write-Host "[1/4] Installing scripts to $ToolsDir ..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $ToolsDir | Out-Null
try {
    Copy-Item "$SrcDir/claude-code-switch.ps1" $CcSwitchPath -Force
    Copy-Item "$SrcDir/claude-code-add.ps1"    $CcAddPath -Force
    Copy-Item "$SrcDir/claude-code-doctor.ps1" $CcDoctorPath -Force
    if ($PlatformWin) {
        Copy-Item "$SrcDir/claude-switch.ps1"  $SwitchPath -Force
        Copy-Item "$SrcDir/claude-add.ps1"     $AddPath -Force
        Copy-Item "$SrcDir/claude-instance-start.ps1" $InstStartPath -Force
        Copy-Item "$SrcDir/claude-instance-stop.ps1"  $InstStopPath -Force
        Copy-Item "$SrcDir/claude-instances-common.ps1" $InstCommonPath -Force
    }
} catch {
    Write-Host "ERROR: could not copy scripts from $SrcDir." -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    return
}

# Record the installed version from the local vendored copy.
try {
    $ver = (Get-Content (Join-Path $PSScriptRoot 'VERSION') -Raw).Trim()
    Set-Content -Path $VersionPath -Value $ver -Encoding ASCII
    Write-Host "      Installed version $ver" -ForegroundColor DarkGray
} catch {}

if ($PlatformWin) {
    # ---------------------------------------------------------------- Windows --
    # 2. Add commands to PowerShell profiles (5.1 + 7)
    Write-Host "[2/4] Adding desktop + Claude Code switch commands to your PowerShell profiles..." -ForegroundColor Cyan
    $docs = [Environment]::GetFolderPath('MyDocuments')   # honours OneDrive redirection
    $profiles = @(
        (Join-Path $docs 'WindowsPowerShell\Microsoft.PowerShell_profile.ps1'),
        (Join-Path $docs 'PowerShell\Microsoft.PowerShell_profile.ps1')
    )
    # Marker-delimited block so re-running the installer cleanly REPLACES the old
    # definitions (lets us add/rename commands on update without leaving dupes).
    $beginMark = '# >>> claude-account-switcher >>>'
    $endMark   = '# <<< claude-account-switcher <<<'
    $func = @"
$beginMark
# Claude account switcher (managed by claude-account-switcher installer)
# Desktop app (claude.ai chats):
function claude-switch-account { & "$SwitchPath" }
function claude-add-account    { & "$AddPath" }
# Claude Code CLI (coding work / usage):
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
        # Remove any previous managed block (and legacy un-marked block), then append fresh.
        $pattern = [Regex]::Escape($beginMark) + '.*?' + [Regex]::Escape($endMark)
        $c = [Regex]::Replace($c, $pattern, '', 'Singleline')
        # Legacy cleanup: drop any old un-delimited block from earlier versions.
        $c = $c -replace '(?s)\r?\n#\s*Claude desktop account switcher[^\r\n]*.*?function\s+claude-add-account\s*\{[^}]*\}', ''
        $c = $c.TrimEnd() + "`r`n`r`n" + $func + "`r`n"
        Set-Content -Path $pf -Value $c -Encoding UTF8
    }

    # 3. Desktop shortcuts (one per switcher)
    Write-Host "[3/4] Creating Desktop shortcuts..." -ForegroundColor Cyan
    $desktop = [Environment]::GetFolderPath('Desktop')
    $psExe   = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'

    $icon = $null
    $pkg  = Get-AppxPackage | Where-Object { $_.PackageFamilyName -eq 'Claude_pzs8sxrjxfjjc' } | Select-Object -First 1
    if ($pkg) {
        $cand = Join-Path $pkg.InstallLocation 'app\Claude.exe'
        if (Test-Path $cand) {
            # Claude Desktop is MSIX: its InstallLocation is version-stamped
            # (...\Claude_<ver>_x64__...) and the whole folder is replaced on
            # every app update. Pointing a shortcut's IconLocation straight into
            # it goes stale (blank/paper icon) the first time Claude updates.
            # Copy the icon to a STABLE path under $ToolsDir so it survives.
            $stableIco = Join-Path $ToolsDir 'claude.ico'
            $srcIco    = Join-Path $pkg.InstallLocation 'app\resources\ion-dist\favicon.ico'
            try {
                if (Test-Path $srcIco) {
                    Copy-Item $srcIco $stableIco -Force
                } else {
                    Add-Type -AssemblyName System.Drawing
                    $ic = [System.Drawing.Icon]::ExtractAssociatedIcon($cand)
                    $fs = [System.IO.File]::Create($stableIco); $ic.Save($fs); $fs.Close(); $ic.Dispose()
                }
            } catch { }
            if (Test-Path $stableIco) { $icon = "$stableIco,0" }
            # The Claude Accounts app reads this for its own window icon - point
            # it at the stable copy too, for the same reason.
            Set-Content -Path (Join-Path $ToolsDir 'claude-exe-path.txt') -Value $stableIco -Encoding UTF8
        }
    }

    $ws = New-Object -ComObject WScript.Shell
    function New-SwitchShortcut($name, $scriptPath, $desc) {
        $sc = $ws.CreateShortcut((Join-Path $desktop $name))
        $sc.TargetPath       = $psExe
        $sc.Arguments        = "-ExecutionPolicy Bypass -NoProfile -File `"$scriptPath`""
        $sc.WorkingDirectory = $HOME
        if ($icon) { $sc.IconLocation = $icon }
        $sc.Description       = $desc
        $sc.WindowStyle      = 1
        $sc.Save()
    }
    New-SwitchShortcut 'Claude Switch Account.lnk' $SwitchPath   'Switch the Claude desktop app account'
    New-SwitchShortcut 'Claude Code Switch.lnk'    $CcSwitchPath 'Switch the Claude Code (CLI) account'

    # Launch shortcuts for the Claude Code CLI (open a console running the command).
    function New-ClaudeLaunchShortcut($name, $cliArgs, $desc) {
        $sc = $ws.CreateShortcut((Join-Path $desktop $name))
        $sc.TargetPath       = $psExe
        $sc.Arguments        = "-NoExit -Command `"claude $cliArgs`""
        $sc.WorkingDirectory = $HOME
        if ($icon) { $sc.IconLocation = $icon }
        $sc.Description       = $desc
        $sc.WindowStyle      = 1
        $sc.Save()
    }
    if (Get-Command claude -ErrorAction SilentlyContinue) {
        New-ClaudeLaunchShortcut 'Claude Code.lnk'          ''           'Launch Claude Code (claude)'
        New-ClaudeLaunchShortcut 'Claude Code Resume.lnk'   '--resume'   'Resume a Claude Code session (claude --resume)'
        New-ClaudeLaunchShortcut 'Claude Code Continue.lnk' '--continue' 'Continue the last Claude Code session (claude --continue)'
    } else {
        Write-Host "      (claude CLI not on PATH - skipped Claude Code launch shortcuts)" -ForegroundColor DarkGray
    }

    # 4. Build & install the Claude Accounts app (concurrent multi-instance
    #    mode) - a normal program window, not a background tray process.
    #    .NET Framework 4.8 WinForms - ships with every Windows 10/11 install,
    #    so the build uses the Framework's own bundled MSBuild.exe rather than
    #    requiring the .NET SDK or any NuGet download.
    Write-Host "[4/4] Building the Claude Accounts app (concurrent multi-instance mode)..." -ForegroundColor Cyan
    $msbuild = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\MSBuild.exe'
    if (-not (Test-Path $msbuild)) { $msbuild = Join-Path $env:WINDIR 'Microsoft.NET\Framework\v4.0.30319\MSBuild.exe' }
    $trayBuilt = $false
    if ((Test-Path $msbuild) -and (Test-Path $GuiSrcDir)) {
        $proj = Join-Path $GuiSrcDir 'ClaudeAccountTray.csproj'
        & $msbuild $proj "/p:Configuration=Release" "/p:OutputPath=$GuiOutDir\" /nologo /verbosity:quiet
        if ($LASTEXITCODE -eq 0 -and (Test-Path (Join-Path $GuiOutDir 'ClaudeAccountTray.exe'))) {
            $trayBuilt = $true
            Write-Host "      Built: $GuiOutDir\ClaudeAccountTray.exe" -ForegroundColor DarkGray
        } else {
            Write-Host "      WARNING: build failed (exit $LASTEXITCODE). Switch/add commands above still work." -ForegroundColor Yellow
        }
    } else {
        Write-Host "      WARNING: MSBuild.exe not found - skipped building the Claude Accounts app." -ForegroundColor Yellow
    }

    if ($trayBuilt) {
        $trayExe = Join-Path $GuiOutDir 'ClaudeAccountTray.exe'
        $sc = $ws.CreateShortcut((Join-Path $desktop 'Claude Accounts.lnk'))
        $sc.TargetPath = $trayExe
        $sc.WorkingDirectory = $GuiOutDir
        if ($icon) { $sc.IconLocation = $icon }
        $sc.Description = 'Run several Claude Desktop accounts at once (hotkeys, dark mode)'
        $sc.Save()
    }

    Write-Host ""
    Write-Host "Done! Claude Account Switcher is installed." -ForegroundColor Green
    Write-Host ""
    Write-Host "Open a NEW PowerShell window so the commands load. Two switchers:" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  DESKTOP APP (claude.ai chats):" -ForegroundColor Gray
    Write-Host "    claude-add-account      save each account (don't use the app's Log out)" -ForegroundColor Gray
    Write-Host "    claude-switch-account   switch (also on the Desktop shortcut)" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  CLAUDE CODE CLI (coding work + usage):" -ForegroundColor Gray
    Write-Host "    claude-code-add         save each account (after /login in Claude Code)" -ForegroundColor Gray
    Write-Host "    claude-code-switch      switch, then 'claude --resume' to continue work" -ForegroundColor Gray
    Write-Host "    claude-code-doctor      check setup (platform, credentials, accounts)" -ForegroundColor Gray
    Write-Host ""
    if ($trayBuilt) {
        Write-Host "  RUN SEVERAL ACCOUNTS AT ONCE (Claude Accounts app):" -ForegroundColor Gray
        Write-Host "    Double-click the 'Claude Accounts' Desktop shortcut - a normal program" -ForegroundColor Gray
        Write-Host "    window (no background tray icon), with per-account hotkeys, dark mode," -ForegroundColor Gray
        Write-Host "    and custom display names. Manual launch only (no autostart at login);" -ForegroundColor Gray
        Write-Host "    closing the window exits the app." -ForegroundColor Gray
        Write-Host ""
    }
    Write-Host "  claude-switch-update      update to the latest version" -ForegroundColor Gray
}
else {
    # ----------------------------------------------------------- macOS / Linux --
    # 2. Add shell functions to the user's shell rc files. Each one invokes the
    #    .ps1 with `pwsh` so the user gets plain `claude-code-switch` commands in
    #    their normal bash/zsh shell.
    Write-Host "[2/2] Adding Claude Code switch commands to your shell (bash/zsh)..." -ForegroundColor Cyan

    if (-not (Get-Command pwsh -ErrorAction SilentlyContinue)) {
        Write-Host "WARNING: 'pwsh' was not found on PATH." -ForegroundColor Yellow
        Write-Host "Install PowerShell (https://aka.ms/powershell) so the commands can run." -ForegroundColor Yellow
    }

    $beginMark = '# >>> claude-account-switcher >>>'
    $endMark   = '# <<< claude-account-switcher <<<'
    $block = @"
$beginMark
# Claude Code account switcher (managed by claude-account-switcher installer)
claude-code-switch() { pwsh -NoProfile -File "$CcSwitchPath"; }
claude-code-add()    { pwsh -NoProfile -File "$CcAddPath"; }
claude-code-doctor() { pwsh -NoProfile -File "$CcDoctorPath"; }
claude-switch-update() { pwsh -NoProfile -Command "Write-Host 'Auto-update is disabled for this vendored copy. Review the diff at https://github.com/karthiknl0/claude-account-switcher then re-run the local installer.' -ForegroundColor Yellow"; }
$endMark
"@

    $rcFiles = @(
        (Join-Path $HOME '.bashrc'),
        (Join-Path $HOME '.zshrc')
    )
    $touched = @()
    foreach ($rc in $rcFiles) {
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
    Write-Host "  claude-switch-update    update to the latest version" -ForegroundColor Gray
}
