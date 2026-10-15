// Running Claude Desktop instances: one per account, told apart only by the
// --user-data-dir argument on their command line (Claude Code's CLI is also
// named claude.exe, so the process name alone means nothing).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex, MutexGuard};
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
// account already). Copying every account's entries into every other
// account's folder makes every session visible - and resumable - from every
// account. Copies, not links: Desktop lstat-checks the folder before each save
// and refuses a junction/symlink ("symlink/file plant"), so a linked list
// loads but never saves again.
// Must run while this instance is stopped: every account's folders (running
// ones too, read only) are merged into the shared folder, newer file wins,
// then the shared folder is merged back into this instance's folders.
// ponytail: a session deleted in one account comes back from the others on
// their next sync - track tombstones if that bites. One folder for all orgs,
// scheduled-tasks.json too - a scheduled task fires in every running instance.
pub fn sync_shared_sessions(user_data_dir: &Path) -> Result<(), String> {
    let shared = crate::store::instances_dir().join("shared-code-sessions");
    fs::create_dir_all(&shared).map_err(|e| e.to_string())?;
    for dir in crate::store::list().iter().filter_map(|a| crate::store::user_data_dir(&a.id).ok()) {
        for org in session_dirs(&dir).into_iter().filter(|o| !is_link(o)) {
            copy_newer(&org, &shared).map_err(|e| format!("merge {}: {e}", org.display()))?;
        }
    }
    for org in session_dirs(user_data_dir) {
        if is_link(&org) {
            // Left by 4.0.0, which linked the folder instead of copying.
            fs::remove_dir(&org).or_else(|_| fs::remove_file(&org)).map_err(|e| e.to_string())?;
            fs::create_dir(&org).map_err(|e| e.to_string())?;
        }
        copy_newer(&shared, &org).map_err(|e| format!("fill {}: {e}", org.display()))?;
    }
    Ok(())
}

// <user-data-dir>/claude-code-sessions/<account>/<org>, links included.
fn session_dirs(user_data_dir: &Path) -> Vec<PathBuf> {
    let Ok(accounts) = fs::read_dir(user_data_dir.join("claude-code-sessions")) else { return vec![] };
    accounts
        .flatten()
        .filter(|a| a.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|a| fs::read_dir(a.path()).ok())
        .flat_map(|orgs| orgs.flatten())
        .filter(|o| o.file_type().is_ok_and(|t| t.is_dir() || t.is_symlink()))
        .map(|o| o.path())
        .collect()
}

// A junction counts too (Windows reports it as a symlink, not a dir).
fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_replaces_link_and_merges_both_ways() {
        let home = std::env::temp_dir().join(format!("ca-sync-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        std::env::set_var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }, &home);
        let shared = crate::store::instances_dir().join("shared-code-sessions");
        let org = |id: &str| crate::store::user_data_dir(id).unwrap().join("claude-code-sessions").join("acc").join("org");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("local_old.json"), "{}").unwrap();
        for id in ["a", "b"] {
            fs::create_dir_all(org(id).parent().unwrap()).unwrap();
            fs::write(crate::store::account_dir(id).unwrap().join("meta.json"), r#"{"email":"x"}"#).unwrap();
        }
        fs::create_dir(org("a")).unwrap();
        fs::write(org("a").join("local_new.json"), "{}").unwrap();
        #[cfg(windows)]
        Command::new("cmd").args(["/c", "mklink", "/J"]).arg(org("b")).arg(&shared).output().unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&shared, org("b")).unwrap();
        assert!(is_link(&org("b")));

        sync_shared_sessions(&crate::store::user_data_dir("b").unwrap()).unwrap();

        assert!(!is_link(&org("b")));
        for f in ["local_old.json", "local_new.json"] {
            assert!(org("b").join(f).exists(), "{f} missing in b");
            assert!(shared.join(f).exists(), "{f} missing in shared");
        }
        fs::remove_dir_all(&home).unwrap();
    }
}
