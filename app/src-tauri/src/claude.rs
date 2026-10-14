// Running Claude Desktop instances: one per account, told apart only by the
// --user-data-dir argument on their command line (Claude Code's CLI is also
// named claude.exe, so the process name alone means nothing).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::SystemTime;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

static PROCESSES: LazyLock<Mutex<System>> = LazyLock::new(|| Mutex::new(System::new()));

// The app polls this every few seconds, so the table is kept and only new
// processes get their command line read (it never changes for a process).
// ponytail: a PID reused within one poll keeps the old command line - fine for
// the status dot; stop() never relies on it (it reads command lines fresh).
pub fn snapshot() -> MutexGuard<'static, System> {
    let mut sys = PROCESSES.lock().unwrap_or_else(|e| e.into_inner());
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::OnlyIfNotSet),
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
    // Fresh, not the cached table: a stale command line on a reused PID would
    // get an unrelated process killed.
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    let own = pids_for(&sys, user_data_dir);
    let has_own_ancestor = |mut pid: Pid| {
        while let Some(parent) = sys.process(pid).and_then(|p| p.parent()) {
            if own.contains(&parent) {
                return true;
            }
            pid = parent;
        }
        false
    };
    if cfg!(target_os = "macos") {
        // Quit the app politely; it takes its helpers down with it.
        for pid in own.iter().filter(|p| !has_own_ancestor(**p)) {
            if let Some(p) = sys.process(*pid) {
                p.kill_with(sysinfo::Signal::Term);
            }
        }
    } else {
        for (pid, p) in sys.processes() {
            if own.contains(pid) || has_own_ancestor(*pid) {
                p.kill();
            }
        }
    }
}

// Starts the account's instance, or - when it already runs - hands the launch
// to it: Electron's single-instance lock is per user-data-dir, so the running
// window just comes to the front.
pub fn launch(user_data_dir: &Path) -> Result<(), String> {
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
fn find_exe() -> Result<PathBuf, String> {
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

// The Code tab's session list lives in
// <user-data-dir>/claude-code-sessions/<account>/<org>/ as one small JSON per
// session that only points at transcripts in ~/.claude (shared by every
// account already). Pointing every account's list folder at one shared folder
// makes every session visible - and resumable - from every account.
// Must run while the instance is stopped. Each real folder is merged in
// (newer file wins), moved to a backup, then replaced by a link.
// ponytail: one folder for all accounts and orgs, scheduled-tasks.json too -
// a scheduled task fires in every running instance; split per org if that bites.
pub fn link_shared_sessions(user_data_dir: &Path) -> Result<(), String> {
    let shared = crate::store::instances_dir().join("shared-code-sessions");
    let root = user_data_dir.join("claude-code-sessions");
    let Ok(accounts) = fs::read_dir(&root) else { return Ok(()) };
    for account in accounts.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())) {
        // A link reports as a symlink, not a dir, so already-linked folders drop out here.
        for org in fs::read_dir(account.path()).map_err(|e| e.to_string())?.flatten() {
            if !org.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let org = org.path();
            fs::create_dir_all(&shared).map_err(|e| e.to_string())?;
            copy_newer(&org, &shared).map_err(|e| format!("merge {}: {e}", org.display()))?;
            let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
            let backup = crate::store::instances_dir()
                .join("session-backup")
                .join(format!("{stamp}-{}", account.file_name().to_string_lossy()));
            fs::create_dir_all(backup.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::rename(&org, &backup).map_err(|e| format!("back up {}: {e}", org.display()))?;
            #[cfg(windows)]
            junction::create(&shared, &org).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&shared, &org).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn copy_newer(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&to)?;
            copy_newer(&entry.path(), &to)?;
            continue;
        }
        let newer = match fs::metadata(&to) {
            Ok(existing) => entry.metadata()?.modified()? > existing.modified()?,
            Err(_) => true,
        };
        if newer {
            fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}
