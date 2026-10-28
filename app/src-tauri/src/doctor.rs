// Check & repair: finds known ways the setup goes wrong. With `fix`, repairs
// only what is safe - stopped accounts only, after a session backup - and
// reports the rest for the user to decide.

use crate::{claude, sessions, store};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
pub struct Finding {
    pub problem: String,
    // What was done, or what the user can do.
    pub action: String,
    pub fixed: bool,
}

fn found(problem: impl Into<String>, action: impl Into<String>, fixed: bool) -> Finding {
    Finding { problem: problem.into(), action: action.into(), fixed }
}

pub fn run(fix: bool) -> Vec<Finding> {
    let mut out = vec![];
    if let Err(e) = claude_installed() {
        out.push(found(e, "Install Claude Desktop from claude.ai/download.", false));
    }

    let sys = claude::snapshot();
    let accounts = store::list();
    let name = |a: &store::Account| a.meta.display_name.clone().unwrap_or_else(|| a.meta.email.clone());

    let mut emails: Vec<String> = accounts.iter().map(|a| a.meta.email.to_lowercase()).collect();
    emails.sort();
    emails.dedup_by(|a, b| {
        let dup = a == b;
        if dup {
            out.push(found(format!("{a} is saved twice"), "Keep one: remove the other's folder from ~/.claude-accounts by hand.", false));
        }
        dup
    });

    let mut keys: Vec<(String, String)> = accounts.iter().filter_map(|a| Some((a.meta.hotkey.clone()?, name(a)))).collect();
    keys.extend(store::settings().best_hotkey.map(|h| (h, "Open best account".into())));
    keys.sort();
    for pair in keys.windows(2).filter(|p| p[0].0 == p[1].0) {
        out.push(found(format!("{} and {} share the hotkey {}", pair[0].1, pair[1].1, pair[0].0), "Change one of them.", false));
    }

    for id in store::list_pending() {
        let Ok(dir) = store::user_data_dir(&id) else { continue };
        let problem = format!("Unfinished add: {id}");
        if claude::is_running(&sys, &dir) {
            out.push(found(problem, "Its Claude window is still open - sign in there or close it.", false));
        } else if crate::token::access_token(&dir).is_ok() {
            out.push(found(problem, "It is signed in - restart Seatdeck to save it.", false));
        } else if fix {
            let done = fs::remove_dir_all(store::account_dir(&id).unwrap_or_default()).is_ok();
            out.push(found(problem, if done { "Removed (never signed in)." } else { "Could not remove it." }, done));
        } else {
            out.push(found(problem, "Never signed in - Repair removes it.", false));
        }
    }

    let presets: Vec<_> = accounts.iter().filter_map(|a| store::user_data_dir(&a.id).ok()).flat_map(|d| sessions::presets(&d)).collect();
    for a in &accounts {
        let Ok(dir) = store::user_data_dir(&a.id) else { continue };
        let running = claude::is_running(&sys, &dir);
        let own = sessions::presets(&dir);
        let mut backed_up = false;
        let mut backup_first = |fixed: &mut bool| {
            if !backed_up {
                backed_up = sessions::backup(&a.id, &dir, None).is_ok();
            }
            *fixed = backed_up;
        };

        if log_has(&dir, "Refusing non-directory") {
            out.push(found(
                format!("{}: Claude refused to save its session list (main.log)", name(a)),
                "Caused by a linked session folder - see the link finding, if any. Old log lines stay after the fix.",
                false,
            ));
        }
        for (_, org, path) in sessions::session_dirs(&dir) {
            if sessions::is_link(&path) {
                let problem = format!("{}: session folder {org} is a link (Claude can't save to it)", name(a));
                let mut fixed = false;
                if fix && !running {
                    backup_first(&mut fixed);
                    fixed = fixed && unlink(&path).is_ok();
                }
                out.push(found(problem, if fixed { "Replaced by a real folder with the same sessions." } else if running { "Stop the account, then Repair." } else { "Repair replaces it by a real folder." }, fixed));
                continue;
            }
            if let Err(e) = sessions::entries(&path) {
                let was_on = store::session_sharing();
                if was_on {
                    let _ = store::update_settings(|s| {
                        s.session_sharing = Some("separate".into());
                        s.session_sharing_stopped = Some(format!("Turned off by Check & repair: {e}"));
                    });
                }
                out.push(found(e, if was_on { "Session sharing was turned off. Nothing was changed in the file." } else { "Sharing stays off. Nothing was changed in the file." }, false));
                continue;
            }
            let foreign = sessions::adopt_presets_in(&path, &own, &presets, true).unwrap_or(0);
            if foreign > 0 {
                let problem = format!("{}: {foreign} session(s) point at another account's saved place", name(a));
                let mut fixed = false;
                if fix && !running {
                    backup_first(&mut fixed);
                    fixed = fixed && sessions::adopt_presets_in(&path, &own, &presets, false).is_ok();
                }
                out.push(found(problem, if fixed { "Pointed at this account's own place." } else if running { "Stop the account, then Repair." } else { "Repair points them at this account's place." }, fixed));
            }
        }
    }
    out
}

// Replaces a linked session folder by a real one holding the same files.
fn unlink(link: &Path) -> std::io::Result<()> {
    let tmp = link.with_extension("seatdeck-tmp");
    fs::create_dir_all(&tmp)?;
    for e in fs::read_dir(link)?.flatten() {
        if e.path().is_file() {
            fs::copy(e.path(), tmp.join(e.file_name()))?;
        }
    }
    fs::remove_dir(link).or_else(|_| fs::remove_file(link))?;
    fs::rename(&tmp, link)
}

// The last MB of the instance's main.log mentions `text`.
fn log_has(user_data_dir: &Path, text: &str) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut log) = fs::File::open(user_data_dir.join("logs").join("main.log")) else { return false };
    let mut tail = vec![];
    let _ = log.seek(SeekFrom::End(-1_000_000)).or_else(|_| log.seek(SeekFrom::Start(0)));
    log.read_to_end(&mut tail).is_ok() && String::from_utf8_lossy(&tail).contains(text)
}

fn claude_installed() -> Result<(), String> {
    #[cfg(windows)]
    return claude::find_exe().map(|_| ());
    #[cfg(not(windows))]
    {
        let home = std::env::home_dir().unwrap_or_default();
        [Path::new("/Applications/Claude.app").to_path_buf(), home.join("Applications/Claude.app")]
            .iter()
            .any(|p| p.exists())
            .then_some(())
            .ok_or_else(|| "Claude Desktop is not installed in /Applications".to_string())
    }
}
