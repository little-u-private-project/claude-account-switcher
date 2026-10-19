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
    // This app is a boundary: when it was started from one of the stopped
    // instance's Code sessions, neither it nor the other accounts' instances
    // it launched go down with it.
    let me = sysinfo::get_current_pid().ok();
    let info = |pid: Pid| (Some(pid) != me).then(|| sys.process(pid)).flatten().map(|p| (p.parent(), p.start_time()));
    let has_own_ancestor = |pid: Pid| has_ancestor_in(pid, &own, info);
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
    let mut all_presets = vec![];
    for dir in crate::store::list().iter().filter_map(|a| crate::store::user_data_dir(&a.id).ok()) {
        for org in session_dirs(&dir).into_iter().filter(|o| !is_link(o)) {
            copy_newer(&org, &shared).map_err(|e| format!("merge {}: {e}", org.display()))?;
        }
        all_presets.extend(presets(&dir));
    }
    let own_presets = presets(user_data_dir);
    for org in session_dirs(user_data_dir) {
        if is_link(&org) {
            // Left by 4.0.0, which linked the folder instead of copying.
            fs::remove_dir(&org).or_else(|_| fs::remove_file(&org)).map_err(|e| e.to_string())?;
            fs::create_dir(&org).map_err(|e| e.to_string())?;
        }
        copy_newer(&shared, &org).map_err(|e| format!("fill {}: {e}", org.display()))?;
        adopt_presets(&org, &own_presets, &all_presets).map_err(|e| format!("presets {}: {e}", org.display()))?;
    }
    Ok(())
}

// Saved places ("presets") in <user-data-dir>/ccd-session-presets.json, as
// (id, target) - target is where it runs, e.g. {"kind":"local","cwd":...}.
fn presets(user_data_dir: &Path) -> Vec<(String, serde_json::Value)> {
    let Ok(text) = fs::read_to_string(user_data_dir.join("ccd-session-presets.json")) else { return vec![] };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return vec![] };
    json["byAccount"]
        .as_object()
        .into_iter()
        .flat_map(|accounts| accounts.values())
        .filter_map(|account| account["presets"].as_array())
        .flatten()
        .filter_map(|p| Some((p["id"].as_str()?.to_string(), p["target"].clone())))
        .collect()
}

// Desktop stamps a new session with the id of the preset it was started from
// and groups the sidebar by it, but every account gives the same folder its
// own random preset id - so a copied session pointed at a preset this account
// doesn't have and fell under "This PC". Point it at this account's preset
// for the same place. The file keeps its mtime so the rewrite never looks
// newer than the real session in the account that owns it.
// ponytail: no preset for that place here -> left as is (still "This PC");
// create the missing preset if that ever comes up.
fn adopt_presets(org: &Path, own: &[(String, serde_json::Value)], all: &[(String, serde_json::Value)]) -> std::io::Result<()> {
    for entry in fs::read_dir(org)?.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("local_") {
            continue;
        }
        let Ok(mut session) = serde_json::from_slice::<serde_json::Value>(&fs::read(entry.path())?) else { continue };
        let mut changed = false;
        for key in ["envScopeId", "startedFromEnvironmentId"] {
            let Some(id) = session[key].as_str() else { continue };
            if own.iter().any(|(own_id, _)| own_id == id) {
                continue;
            }
            let Some((_, place)) = all.iter().find(|(other, _)| other == id) else { continue };
            if let Some((own_id, _)) = own.iter().find(|(_, own_place)| own_place == place) {
                session[key] = own_id.clone().into();
                changed = true;
            }
        }
        if changed {
            let modified = entry.metadata()?.modified()?;
            fs::write(entry.path(), serde_json::to_vec(&session)?)?;
            fs::File::options().write(true).open(entry.path())?.set_modified(modified)?;
        }
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
        fs::write(org("a").join("local_new.json"), r#"{"envScopeId":"pa","startedFromEnvironmentId":"builtin_local"}"#).unwrap();
        for (id, preset) in [("a", "pa"), ("b", "pb")] {
            let presets = format!(r#"{{"byAccount":{{"u":{{"presets":[{{"id":"{preset}","target":{{"kind":"local","cwd":"F:\\w"}}}}]}}}}}}"#);
            fs::write(crate::store::user_data_dir(id).unwrap().join("ccd-session-presets.json"), presets).unwrap();
        }
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
        let copied: serde_json::Value = serde_json::from_slice(&fs::read(org("b").join("local_new.json")).unwrap()).unwrap();
        assert_eq!(copied["envScopeId"], "pb", "a's preset id not mapped to b's");
        assert_eq!(copied["startedFromEnvironmentId"], "builtin_local");
        let modified = |dir: PathBuf| fs::metadata(dir.join("local_new.json")).unwrap().modified().unwrap();
        assert_eq!(modified(org("b")), modified(shared.clone()), "rewrite changed the mtime");
        fs::remove_dir_all(&home).unwrap();
    }
}
