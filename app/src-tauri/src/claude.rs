// Running Claude Desktop instances: one per account, told apart only by the
// --user-data-dir argument on their command line (Claude Code's CLI is also
// named claude.exe, so the process name alone means nothing).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

// Fresh every time (~15 ms for ~500 processes): a kept table holds on to a
// dead process's command line when Windows hands its PID to a new one, so a
// stopped account kept showing "Running" (and Open skipped the session sync).
pub fn snapshot() -> System {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    sys
}

fn same_path(a: &str, b: &Path) -> bool {
    let a = a.trim_matches('"').trim_end_matches(['/', '\\']);
    let b = b.to_string_lossy();
    let b = b.trim_end_matches(['/', '\\']);
    if cfg!(windows) { a.eq_ignore_ascii_case(b) } else { a == b }
}

fn pids_for(sys: &System, user_data_dir: &Path) -> HashSet<Pid> {
    sys.processes()
        .iter()
        .filter(|(_, p)| {
            p.cmd().iter().any(|arg| {
                arg.to_string_lossy()
                    .strip_prefix("--user-data-dir=")
                    .is_some_and(|v| same_path(v, user_data_dir))
            })
        })
        .map(|(pid, _)| *pid)
        .collect()
}

pub fn is_running(sys: &System, user_data_dir: &Path) -> bool {
    !pids_for(sys, user_data_dir).is_empty()
}

// Kills the instance and everything it started (Code sessions included).
pub fn stop(user_data_dir: &Path) {
    let sys = snapshot();
    let own = pids_for(&sys, user_data_dir);
    if cfg!(target_os = "macos") {
        // Quit the app politely; it takes its helpers down with it.
        let info = |pid: Pid| sys.process(pid).map(|p| (p.parent(), p.start_time()));
        for pid in own.iter().filter(|p| !has_ancestor_in(**p, &own, info)) {
            if let Some(p) = sys.process(*pid) {
                p.kill_with(sysinfo::Signal::Term);
            }
        }
    } else {
        kill_trees(&sys, &own);
    }
}

fn kill_trees(sys: &System, own: &HashSet<Pid>) {
    // This app is a boundary: when it was started from one of the stopped
    // instance's Code sessions, neither it nor the other accounts' instances
    // it launched go down with it.
    let me = sysinfo::get_current_pid().ok();
    let info = |pid: Pid| (Some(pid) != me).then(|| sys.process(pid)).flatten().map(|p| (p.parent(), p.start_time()));
    for (pid, p) in sys.processes() {
        if own.contains(pid) || has_ancestor_in(*pid, own, info) {
            p.kill();
        }
    }
}

// Claude Desktop's last log line when it quits to install an update.
pub fn quit_for_update(user_data_dir: &Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut log) = fs::File::open(user_data_dir.join("logs").join("main.log")) else { return false };
    let mut tail = vec![];
    let _ = log.seek(SeekFrom::End(-4096)).or_else(|_| log.seek(SeekFrom::Start(0)));
    log.read_to_end(&mut tail).is_ok() && String::from_utf8_lossy(&tail).contains("going down for update")
}

// Main processes of Claude Desktop on its own default profile (no
// --user-data-dir) started at or after `since` (unix seconds).
#[cfg(windows)]
pub fn default_instances(sys: &System, since: u64) -> HashSet<Pid> {
    let Ok(exe) = find_exe() else { return HashSet::new() };
    sys.processes()
        .iter()
        .filter(|(_, p)| {
            let cmd = p.cmd();
            p.start_time() >= since
                && cmd.first().is_some_and(|a| same_path(&a.to_string_lossy(), &exe))
                && !cmd.iter().any(|a| {
                    let a = a.to_string_lossy();
                    a.starts_with("--user-data-dir=") || a.starts_with("--type=")
                })
        })
        .map(|(pid, _)| *pid)
        .collect()
}

#[cfg(windows)]
pub fn stop_default(since: u64) {
    let sys = snapshot();
    kill_trees(&sys, &default_instances(&sys, since));
}

// Walks up the parent chain; info(pid) = (parent, start time). Windows reuses
// a dead parent's PID, so a "parent" that started after its child is a
// stranger - without this check Stop killed unrelated processes (this app,
// explorer) whose dead parent's PID now belonged to the stopped instance.
fn has_ancestor_in(mut pid: Pid, own: &HashSet<Pid>, info: impl Fn(Pid) -> Option<(Option<Pid>, u64)>) -> bool {
    while let Some((Some(parent), started)) = info(pid) {
        if info(parent).is_none_or(|(_, parent_started)| parent_started > started) {
            return false;
        }
        if own.contains(&parent) {
            return true;
        }
        pid = parent;
    }
    false
}

// Starts the account's instance, or - when it already runs - hands the launch
// to it: on Windows Electron's single-instance lock is per user-data-dir, so the
// running window just comes to the front. Claude on macOS has no such lock
// (`open -n` would start a second copy on the same profile), so bring the
// running one forward ourselves.
pub fn launch(user_data_dir: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
        let sys = snapshot();
        let own = pids_for(&sys, user_data_dir);
        let info = |pid: Pid| sys.process(pid).map(|p| (p.parent(), p.start_time()));
        // The app itself, not its helpers (they carry --user-data-dir too).
        // ponytail: activating only raises existing windows - an instance whose
        // last window was closed stays windowless (needs a reopen Apple Event).
        if let Some(main) = own.iter().find(|p| !has_ancestor_in(**p, &own, info)) {
            if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(main.as_u32() as i32) {
                app.unhide();
                app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
                return Ok(());
            }
        }
    }
    let arg = format!("--user-data-dir={}", user_data_dir.display());
    #[cfg(windows)]
    {
        Command::new(find_exe()?).arg(arg).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        let app = ["/Applications/Claude.app".into(), std::env::home_dir().unwrap_or_default().join("Applications/Claude.app")]
            .into_iter()
            .find(|p: &PathBuf| p.exists())
            .ok_or("Claude Desktop is not installed in /Applications")?;
        Command::new("open").arg("-n").arg("-a").arg(app).arg("--args").arg(arg).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn find_exe() -> Result<PathBuf, String> {
    use windows::core::HSTRING;
    use windows::Management::Deployment::PackageManager;

    // Microsoft Store / MSIX install (the usual one).
    let msix = (|| -> windows::core::Result<Option<PathBuf>> {
        let packages = PackageManager::new()?
            .FindPackagesByUserSecurityIdPackageFamilyName(&HSTRING::new(), &HSTRING::from("Claude_pzs8sxrjxfjjc"))?;
        for p in packages {
            let exe = PathBuf::from(p.InstalledPath()?.to_string()).join("app").join("Claude.exe");
            if exe.exists() {
                return Ok(Some(exe));
            }
        }
        Ok(None)
    })()
    .ok()
    .flatten();
    if let Some(exe) = msix {
        return Ok(exe);
    }
    // Direct-download installer.
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default());
    [local.join("AnthropicClaude").join("claude.exe"), local.join("Programs").join("Claude").join("Claude.exe")]
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| "Claude Desktop is not installed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_parent_pid_is_not_an_ancestor() {
        // pid -> (parent, start time); 1 is the stopped instance.
        let table = std::collections::HashMap::from([
            (1, (None, 100)),
            (2, (Some(1), 101)), // real child
            (3, (Some(2), 102)), // real grandchild
            (4, (Some(5), 50)),  // its parent died, PID 5 was reused...
            (5, (Some(1), 105)), // ...by a younger child of the instance
            (6, (Some(99), 50)), // parent gone
        ]);
        let info = |p: Pid| table.get(&p.as_u32()).map(|(parent, t)| (parent.map(Pid::from_u32), *t));
        let own = HashSet::from([Pid::from_u32(1)]);
        let hit = |p| has_ancestor_in(Pid::from_u32(p), &own, &info);
        assert!(hit(2) && hit(3) && hit(5));
        assert!(!hit(4) && !hit(6) && !hit(1));
    }

    #[test]
    fn spots_quit_for_update() {
        let dir = std::env::temp_dir().join(format!("ca-update-{}", std::process::id()));
        fs::create_dir_all(dir.join("logs")).unwrap();
        let log = dir.join("logs").join("main.log");
        assert!(!quit_for_update(&dir), "no log");
        fs::write(&log, "x\n".repeat(5000) + "[info] beforeQuitForUpdate handler fired, going down for update\n").unwrap();
        assert!(quit_for_update(&dir));
        fs::write(&log, "[info] beforeQuitForUpdate handler fired, going down for update\n[info] Starting app {\n".to_string() + &"x\n".repeat(5000)).unwrap();
        assert!(!quit_for_update(&dir), "relaunched since");
        fs::remove_dir_all(&dir).unwrap();
    }
}
