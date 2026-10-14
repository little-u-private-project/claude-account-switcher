<#
    Claude Account Switcher - uninstaller (Claude Code CLI switcher, plus any
    leftovers of the retired desktop switcher / WinForms app)

      .\uninstall.ps1

    The Claude Accounts app has its own uninstaller (Windows: Settings > Apps;
    macOS: drag it to the Trash).

    Does NOT touch your saved accounts (~/.claude-accounts), the shared session
    list (~/.claude-instances), or your Claude login. Remove those manually if
    you want - but each account folder holds that account's whole signed-in
    Claude Desktop profile:
      Remove-Item -Recurse -Force ~/.claude-accounts, ~/.claude-instances
#>

Write-Host "Removing Claude Account Switcher..." -ForegroundColor Cyan

$ToolsDir = Join-Path $HOME '.claude-tools'
if (Test-Path $ToolsDir) { Remove-Item -Recurse -Force $ToolsDir; Write-Host " - removed $ToolsDir" }

$beginMark = '# >>> claude-account-switcher >>>'
$endMark   = '# <<< claude-account-switcher <<<'
$pattern   = [Regex]::Escape($beginMark) + '.*?' + [Regex]::Escape($endMark)

$onWindows = -not (Test-Path variable:IsWindows) -or $IsWindows
if ($onWindows) {
    Get-Process -Name 'ClaudeAccountTray' -ErrorAction SilentlyContinue |
        ForEach-Object { $_.Kill(); $_.WaitForExit(5000) | Out-Null }
    $desktop = [Environment]::GetFolderPath('Desktop')
    $gone = @((Join-Path $env:LOCALAPPDATA 'ClaudeAccountTray')) +
        (@('Claude Switch Account.lnk', 'Claude Code Switch.lnk', 'Claude Code.lnk', 'Claude Code Resume.lnk', 'Claude Code Continue.lnk') |
            ForEach-Object { Join-Path $desktop $_ })
    # 'Claude Accounts.lnk' may be the new app's own shortcut - only the old one goes.
    $oldLnk = Join-Path $desktop 'Claude Accounts.lnk'
    if ((Test-Path $oldLnk) -and (New-Object -ComObject WScript.Shell).CreateShortcut($oldLnk).TargetPath -like '*ClaudeAccountTray*') { $gone += $oldLnk }
    foreach ($p in $gone) {
        if (-not (Test-Path $p)) { continue }
        try { Remove-Item -Recurse -Force $p -ErrorAction Stop; Write-Host " - removed $p" }
        catch { Write-Host " - could not remove $p - delete it by hand ($($_.Exception.Message))" -ForegroundColor Yellow }
    }

    $docs = [Environment]::GetFolderPath('MyDocuments')
    $rcFiles = @(
        (Join-Path $docs 'WindowsPowerShell\Microsoft.PowerShell_profile.ps1'),
        (Join-Path $docs 'PowerShell\Microsoft.PowerShell_profile.ps1')
    )
} else {
    $rcFiles = @((Join-Path $HOME '.bashrc'), (Join-Path $HOME '.zshrc'))
}

foreach ($rc in $rcFiles) {
    if (-not (Test-Path $rc)) { continue }
    $c = Get-Content $rc -Raw
    if ($null -eq $c -or $c -notmatch [Regex]::Escape($beginMark)) { continue }
    $c = [Regex]::Replace($c, $pattern, '', 'Singleline').TrimEnd() + [Environment]::NewLine
    [System.IO.File]::WriteAllText($rc, $c, (New-Object System.Text.UTF8Encoding($false)))
    Write-Host " - cleaned $rc"
}

Write-Host "Done. Open a new terminal for changes to take effect." -ForegroundColor Green
