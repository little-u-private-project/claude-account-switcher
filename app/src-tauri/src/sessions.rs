// Optional sharing of the Code tab's session list between accounts.
//
// The list lives in <user-data-dir>/claude-code-sessions/<account>/<org>/ as
// one small local_*.json per session that only points at a transcript in
// ~/.claude (shared by every account already). Copying every account's
// entries into every other account's folder makes every session visible - and
// resumable - from every account. Copies, not links: Desktop lstat-checks the
// folder before each save and refuses a junction/symlink ("symlink/file
// plant"), so a linked list loads but never saves again.
//
// Hub: ~/.claude-instances/shared-code-sessions/<org>/ - one per org, so orgs
// never mix - plus sync-state.json (what each folder held after the last sync,
// and tombstones for deleted sessions). Only local_*.json is shared; the other
// files Desktop keeps there (scheduled-tasks.json, backlog/, ...) stay per
// account, so a scheduled task no longer fires in every instance.
//
// Runs when an account is opened while stopped: every account's folders are
// read (running ones too, never written), merged into the hub (newer file
// wins), then the hub is merged into the opened account's folders. A snapshot
// of that account's list and the hub is taken first (session-backup/).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const KEEP_BACKUPS: usize = 10;

pub fn hub() -> PathBuf {
    crate::store::instances_dir().join("shared-code-sessions")
}

fn backups() -> PathBuf {
    crate::store::instances_dir().join("session-backup")
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct OrgState {
    // session file name -> when its delete was noticed (unix ms)
    #[serde(default)]
    tombstones: BTreeMap<String, u64>,
    // "<account id>/<account uuid>" -> session file names after the last sync
    #[serde(default)]
    synced: BTreeMap<String, BTreeSet<String>>,
}

type State = BTreeMap<String, OrgState>;

fn state_path() -> PathBuf {
    hub().join("sync-state.json")
}

fn load_state() -> State {
    fs::read(state_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_state(state: &State) -> Result<(), String> {
    fs::write(state_path(), serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

// What a session entry must look like for copying it to be safe. A Claude
// Desktop update that changes the format stops sync instead of spreading
// files this app no longer understands.
pub fn expected_shape(v: &Value) -> bool {
    v.is_object() && v["cliSessionId"].is_string() && v["cwd"].is_string()
}

pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub modified: u64,
    // false = not valid JSON (Desktop may be writing it right now): counts as
    // present, but is not copied this time.
    pub readable: bool,
}

// The local_*.json entries of one session folder. Err = an entry of an
// unexpected shape.
pub fn entries(dir: &Path) -> Result<Vec<Entry>, String> {
    let mut out = vec![];
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("local_") && name.ends_with(".json")) || !e.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let modified = e.metadata().and_then(|m| m.modified()).map(ms).unwrap_or(0);
        let parsed = fs::read(e.path()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok());
        if parsed.as_ref().is_some_and(|v| !expected_shape(v)) {
            return Err(format!("{} has an unexpected format", e.path().display()));
        }
        out.push(Entry { name, path: e.path(), modified, readable: parsed.is_some() });
    }
    Ok(out)
}

// (account uuid, org id, path) of <user-data-dir>/claude-code-sessions/<account>/<org>, links included.
pub fn session_dirs(user_data_dir: &Path) -> Vec<(String, String, PathBuf)> {
    let Ok(accounts) = fs::read_dir(user_data_dir.join("claude-code-sessions")) else { return vec![] };
    let mut out = vec![];
    for a in accounts.flatten().filter(|a| a.file_type().is_ok_and(|t| t.is_dir())) {
        for o in fs::read_dir(a.path()).into_iter().flatten().flatten() {
            if o.file_type().is_ok_and(|t| t.is_dir() || t.is_symlink()) {
                out.push((a.file_name().to_string_lossy().into_owned(), o.file_name().to_string_lossy().into_owned(), o.path()));
            }
        }
    }
    out
}

// A junction counts too (Windows reports it as a symlink, not a dir).
pub fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

// Copies keep the source's mtime: "newer wins" and tombstones compare it.
fn copy_file(from: &Path, to: &Path) -> std::io::Result<()> {
    let modified = fs::metadata(from)?.modified()?;
    fs::copy(from, to)?;
    fs::File::options().write(true).open(to)?.set_modified(modified)
}

fn copy_if_newer(from: &Path, to: &Path) -> std::io::Result<()> {
    let newer = match fs::metadata(to) {
        Ok(existing) => fs::metadata(from)?.modified()? > existing.modified()?,
        Err(_) => true,
    };
    if newer {
        copy_file(from, to)?;
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)?.flatten() {
        let dst = to.join(e.file_name());
        if e.path().is_dir() {
            copy_tree(&e.path(), &dst)?;
        } else {
            copy_file(&e.path(), &dst)?;
        }
    }
    Ok(())
}

// Turns sharing off and says why; returns the reason as the error.
fn stop_sharing(reason: String) -> String {
    let reason = format!("Session sharing turned itself off: {reason}. Claude Desktop may have changed its file format - nothing was copied.");
    let _ = crate::store::update_settings(|s| {
        s.session_sharing = Some("separate".into());
        s.session_sharing_stopped = Some(reason.clone());
    });
    reason
}

// `target` = the account being opened; it must be stopped.
pub fn sync(target: &str) -> Result<(), String> {
    let target_dir = crate::store::user_data_dir(target)?;
    // Read and check everything first, so a format problem stops before anything is written.
    let mut folders = vec![];
    for a in crate::store::list() {
        let Ok(dir) = crate::store::user_data_dir(&a.id) else { continue };
        for (uuid, org, path) in session_dirs(&dir) {
            if is_link(&path) {
                continue; // 4.0.0 linked the folder - the target's link is replaced below, others are skipped
            }
            let found = entries(&path).map_err(stop_sharing)?;
            folders.push((format!("{}/{uuid}", a.id), a.id == target, org, path, found));
        }
    }

    let hub = hub();
    fs::create_dir_all(&hub).map_err(|e| e.to_string())?;
    backup(target, &target_dir, None)?;
    migrate_flat_hub(&folders.iter().map(|(_, _, org, path, _)| (org.clone(), path.clone())).collect::<Vec<_>>())?;

    let mut state = load_state();
    let now = now_ms();
    for (key, is_target, org, _, found) in &folders {
        let st = state.entry(org.clone()).or_default();
        let hub_org = hub.join(org);
        fs::create_dir_all(&hub_org).map_err(|e| e.to_string())?;
        let present: BTreeSet<String> = found.iter().map(|e| e.name.clone()).collect();
        // Synced here before and gone now = deleted in this account. An empty
        // folder is not read as "deleted everything" (new sign-in, reset).
        if let Some(before) = st.synced.get(key).filter(|_| !present.is_empty()) {
            for gone in before.difference(&present) {
                st.tombstones.insert(gone.clone(), now);
                let _ = fs::remove_file(hub_org.join(gone));
            }
        }
        for e in found.iter().filter(|e| e.readable) {
            if let Some(&deleted) = st.tombstones.get(&e.name) {
                if e.modified <= deleted {
                    continue;
                }
                st.tombstones.remove(&e.name); // used again since the delete
            }
            copy_if_newer(&e.path, &hub_org.join(&e.name)).map_err(|err| format!("merge {}: {err}", e.path.display()))?;
        }
        if !is_target {
            st.synced.insert(key.clone(), present);
        }
    }

    let all_presets: Vec<_> = crate::store::list()
        .iter()
        .filter_map(|a| crate::store::user_data_dir(&a.id).ok())
        .flat_map(|d| presets(&d))
        .collect();
    let own_presets = presets(&target_dir);
    for (uuid, org, path) in session_dirs(&target_dir) {
        if is_link(&path) {
            // Left by 4.0.0, which linked the folder instead of copying.
            fs::remove_dir(&path).or_else(|_| fs::remove_file(&path)).map_err(|e| e.to_string())?;
            fs::create_dir(&path).map_err(|e| e.to_string())?;
        }
        let st = state.entry(org.clone()).or_default();
        let hub_org = hub.join(&org);
        for e in entries(&hub_org).unwrap_or_default().iter().filter(|e| e.readable) {
            copy_if_newer(&e.path, &path.join(&e.name)).map_err(|err| format!("fill {}: {err}", path.display()))?;
        }
        for e in entries(&path).map_err(stop_sharing)? {
            if st.tombstones.get(&e.name).is_some_and(|&deleted| e.modified <= deleted) {
                fs::remove_file(&e.path).map_err(|err| err.to_string())?;
            }
        }
        adopt_presets(&path, &own_presets, &all_presets).map_err(|e| format!("presets {}: {e}", path.display()))?;
        let present = entries(&path).unwrap_or_default().into_iter().map(|e| e.name).collect();
        st.synced.insert(format!("{target}/{uuid}"), present);
    }
    save_state(&state)
}

// Before 4.3.0 the hub was one flat folder for every org, with
// scheduled-tasks.json and Desktop's other files in it. Each entry moves to
// the org of the account folder that has it (the only org if there is just
// one), the rest is dropped - the backup taken just before keeps it all.
fn migrate_flat_hub(folders: &[(String, PathBuf)]) -> Result<(), String> {
    let hub = hub();
    let orgs: BTreeSet<&String> = folders.iter().map(|(org, _)| org).collect();
    for e in fs::read_dir(&hub).map_err(|e| e.to_string())?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        if name == "sync-state.json" || (path.is_dir() && !matches!(name.as_str(), "backlog" | "waiting-input")) {
            continue; // an org folder
        }
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
            continue;
        }
        let org = folders
            .iter()
            .find(|(_, dir)| dir.join(&name).exists())
            .map(|(org, _)| org)
            .or((orgs.len() == 1).then(|| *orgs.first().unwrap()));
        match org.filter(|_| name.starts_with("local_")) {
            Some(org) => {
                let dst = hub.join(org);
                fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
                copy_if_newer(&path, &dst.join(&name)).map_err(|e| e.to_string())?;
                fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
            None => fs::remove_file(&path).map_err(|e| e.to_string())?,
        }
    }
    Ok(())
}

// Saved places ("presets") in <user-data-dir>/ccd-session-presets.json, as
// (id, target) - target is where it runs, e.g. {"kind":"local","cwd":...}.
pub fn presets(user_data_dir: &Path) -> Vec<(String, Value)> {
    let Ok(text) = fs::read_to_string(user_data_dir.join("ccd-session-presets.json")) else { return vec![] };
    let Ok(json) = serde_json::from_str::<Value>(&text) else { return vec![] };
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
// newer than the real session in the account that owns it. Returns how many
// entries were (or, with dry_run, would be) rewritten.
// ponytail: no preset for that place here -> left as is (still "This PC");
// create the missing preset if that ever comes up.
pub fn adopt_presets_in(org: &Path, own: &[(String, Value)], all: &[(String, Value)], dry_run: bool) -> std::io::Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(org)?.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("local_") {
            continue;
        }
        let Ok(mut session) = serde_json::from_slice::<Value>(&fs::read(entry.path())?) else { continue };
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
            count += 1;
            if !dry_run {
                let modified = entry.metadata()?.modified()?;
                fs::write(entry.path(), serde_json::to_vec(&session)?)?;
                fs::File::options().write(true).open(entry.path())?.set_modified(modified)?;
            }
        }
    }
    Ok(count)
}

fn adopt_presets(org: &Path, own: &[(String, Value)], all: &[(String, Value)]) -> std::io::Result<usize> {
    adopt_presets_in(org, own, all, false)
}

#[derive(Serialize)]
pub struct Backup {
    pub stamp: String,
    pub account: String,
}

// Snapshot of one account's session list and the hub:
// session-backup/<unix ms>/{account,hub,snapshot.json}. Keeps the newest 10
// snapshots made this way (older folders without snapshot.json are left alone).
pub fn backup(account: &str, user_data_dir: &Path, keep: Option<&str>) -> Result<(), String> {
    let mut stamp = now_ms();
    while backups().join(stamp.to_string()).exists() {
        stamp += 1;
    }
    let dir = backups().join(stamp.to_string());
    let sessions = user_data_dir.join("claude-code-sessions");
    let run = || -> std::io::Result<()> {
        fs::create_dir_all(&dir)?;
        if sessions.exists() {
            copy_tree(&sessions, &dir.join("account"))?;
        }
        if hub().exists() {
            copy_tree(&hub(), &dir.join("hub"))?;
        }
        fs::write(dir.join("snapshot.json"), serde_json::to_vec(&serde_json::json!({ "account": account }))?)
    };
    run().map_err(|e| format!("backup failed, nothing was synced: {e}"))?;
    for old in list_backups().into_iter().skip(KEEP_BACKUPS).filter(|b| Some(b.stamp.as_str()) != keep) {
        let _ = fs::remove_dir_all(backups().join(old.stamp));
    }
    Ok(())
}

// Newest first.
pub fn list_backups() -> Vec<Backup> {
    let mut out: Vec<Backup> = fs::read_dir(backups())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let stamp = e.file_name().to_string_lossy().into_owned();
            let meta: Value = serde_json::from_slice(&fs::read(e.path().join("snapshot.json")).ok()?).ok()?;
            Some(Backup { account: meta["account"].as_str()?.to_string(), stamp })
        })
        .collect();
    out.sort_by_key(|b| std::cmp::Reverse(b.stamp.parse::<u64>().unwrap_or(0)));
    out
}

// Puts a snapshot back: that account's session list and the hub. The account
// must be stopped (checked by the caller). The current state is snapshotted
// first, so a restore can be undone. Afterwards no delete is inferred from the
// restored lists, and sessions in them are no longer tombstoned.
pub fn restore(stamp: &str) -> Result<String, String> {
    let snap = list_backups().into_iter().find(|b| b.stamp == stamp).ok_or("no such backup")?;
    let src = backups().join(&snap.stamp);
    let dir = crate::store::user_data_dir(&snap.account)?;
    backup(&snap.account, &dir, Some(stamp))?;
    let replace = |from: PathBuf, to: PathBuf| -> std::io::Result<()> {
        if from.exists() {
            if to.exists() {
                fs::remove_dir_all(&to)?;
            }
            copy_tree(&from, &to)?;
        }
        Ok(())
    };
    replace(src.join("account"), dir.join("claude-code-sessions")).map_err(|e| e.to_string())?;
    replace(src.join("hub"), hub()).map_err(|e| e.to_string())?;
    let mut state = load_state();
    for (org, st) in state.iter_mut() {
        st.synced.clear();
        let restored: BTreeSet<String> = session_dirs(&dir)
            .into_iter()
            .filter(|(_, o, _)| o == org)
            .map(|(_, _, path)| path)
            .chain([hub().join(org)])
            .flat_map(|path| entries(&path).unwrap_or_default())
            .map(|e| e.name)
            .collect();
        st.tombstones.retain(|name, _| !restored.contains(name));
    }
    if hub().exists() {
        save_state(&state)?;
    }
    Ok(snap.account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // One test, because it points HOME at a temp folder for the whole process.
    #[test]
    fn sync_end_to_end() {
        let home = std::env::temp_dir().join(format!("seatdeck-sync-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        std::env::set_var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }, &home);
        let org = |id: &str| crate::store::user_data_dir(id).unwrap().join("claude-code-sessions").join(format!("u-{id}")).join("org");
        let session = |preset: &str| format!(r#"{{"cliSessionId":"c","cwd":"F:\\w","envScopeId":"{preset}","startedFromEnvironmentId":"builtin_local"}}"#);
        let write = |path: PathBuf, body: &str| {
            fs::write(&path, body).unwrap();
            // mtimes must differ from "now" for the newer/tombstone checks
            fs::File::options().write(true).open(&path).unwrap().set_modified(SystemTime::now() - Duration::from_secs(60)).unwrap();
        };
        for (id, preset) in [("a", "pa"), ("b", "pb")] {
            fs::create_dir_all(org(id).parent().unwrap()).unwrap();
            fs::write(crate::store::account_dir(id).unwrap().join("meta.json"), r#"{"email":"x"}"#).unwrap();
            let presets = format!(r#"{{"byAccount":{{"u":{{"presets":[{{"id":"{preset}","target":{{"kind":"local","cwd":"F:\\w"}}}}]}}}}}}"#);
            fs::write(crate::store::user_data_dir(id).unwrap().join("ccd-session-presets.json"), presets).unwrap();
        }
        // A 4.2 flat hub: an entry b's folder has, Desktop's other files.
        let flat = hub();
        fs::create_dir_all(flat.join("backlog")).unwrap();
        write(flat.join("local_old.json"), &session("pb"));
        write(flat.join("scheduled-tasks.json"), "{}");
        fs::create_dir(org("a")).unwrap();
        write(org("a").join("local_new.json"), &session("pa"));
        write(org("a").join("scheduled-tasks.json"), "{}");
        // b's folder is a 4.0.0 junction to the hub.
        #[cfg(windows)]
        std::process::Command::new("cmd").args(["/c", "mklink", "/J"]).arg(org("b")).arg(&flat).output().unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&flat, org("b")).unwrap();
        assert!(is_link(&org("b")));

        sync("b").unwrap();

        assert!(!is_link(&org("b")));
        let hub_org = hub().join("org");
        for f in ["local_old.json", "local_new.json"] {
            assert!(org("b").join(f).exists(), "{f} missing in b");
            assert!(hub_org.join(f).exists(), "{f} missing in hub/org");
        }
        assert!(!flat.join("local_old.json").exists() && !flat.join("backlog").exists(), "flat hub not migrated");
        assert!(!hub_org.join("scheduled-tasks.json").exists() && !org("b").join("scheduled-tasks.json").exists());
        let copied: Value = serde_json::from_slice(&fs::read(org("b").join("local_new.json")).unwrap()).unwrap();
        assert_eq!(copied["envScopeId"], "pb", "a's preset id not mapped to b's");
        assert_eq!(copied["startedFromEnvironmentId"], "builtin_local");
        let modified = |dir: PathBuf| fs::metadata(dir.join("local_new.json")).unwrap().modified().unwrap();
        assert_eq!(modified(org("b")), modified(hub_org.clone()), "rewrite changed the mtime");
        assert_eq!(list_backups().len(), 1);

        // Delete in b; opening a (b's list is read, a is filled) must not bring it back.
        sync("a").unwrap();
        assert!(org("a").join("local_old.json").exists());
        fs::remove_file(org("b").join("local_new.json")).unwrap();
        sync("a").unwrap();
        assert!(!hub_org.join("local_new.json").exists(), "delete did not reach the hub");
        assert!(!org("a").join("local_new.json").exists(), "deleted session still in a");
        sync("b").unwrap();
        assert!(!org("b").join("local_new.json").exists(), "deleted session came back");

        // Restore the snapshot taken before the delete reached a.
        let before = list_backups().into_iter().find(|b| b.account == "a" && {
            backups().join(&b.stamp).join("account").join("u-a").join("org").join("local_new.json").exists()
        });
        assert_eq!(restore(&before.unwrap().stamp).unwrap(), "a");
        assert!(org("a").join("local_new.json").exists(), "restore did not bring it back");
        sync("a").unwrap();
        assert!(org("a").join("local_new.json").exists(), "restored session deleted again");

        // At most 10 snapshots.
        for _ in 0..12 {
            sync("a").unwrap();
        }
        assert_eq!(list_backups().len(), KEEP_BACKUPS);

        // An entry of an unknown format stops sharing instead of spreading it.
        crate::store::update_settings(|s| s.session_sharing = Some("shared".into())).unwrap();
        write(org("b").join("local_weird.json"), r#"{"id":1}"#);
        assert!(sync("a").is_err());
        assert!(!crate::store::session_sharing());
        assert!(crate::store::settings().session_sharing_stopped.is_some());
        fs::remove_dir_all(&home).unwrap();
    }
}
