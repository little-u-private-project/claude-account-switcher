// Saved accounts: ~/.claude-accounts/<id>/meta.json, Claude's own data in
// ~/.claude-accounts/<id>/instance (its --user-data-dir). Same layout the
// PowerShell tool wrote, so existing accounts keep working untouched.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,
    // Fields older versions wrote (savedAt, sourceRoot, ...) survive a rewrite.
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

pub struct Account {
    pub id: String,
    pub meta: Meta,
}

fn home() -> PathBuf {
    std::env::home_dir().expect("no home directory")
}

pub fn accounts_dir() -> PathBuf {
    home().join(".claude-accounts")
}

pub fn instances_dir() -> PathBuf {
    home().join(".claude-instances")
}

// The id comes from the UI; never let it escape the accounts folder.
pub fn account_dir(id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id == "." || id == ".." || id.contains(['/', '\\']) {
        return Err(format!("invalid account id: {id}"));
    }
    Ok(accounts_dir().join(id))
}

pub fn user_data_dir(id: &str) -> Result<PathBuf, String> {
    Ok(account_dir(id)?.join("instance"))
}

fn read_meta(id: &str) -> Option<Meta> {
    let text = fs::read_to_string(account_dir(id).ok()?.join("meta.json")).ok()?;
    // PowerShell 5.1 writes UTF-8 with a BOM.
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

pub fn list() -> Vec<Account> {
    let mut out: Vec<Account> = fs::read_dir(accounts_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            read_meta(&id).map(|meta| Account { id, meta })
        })
        .collect();
    out.sort_by_key(|a| a.meta.email.to_lowercase());
    out
}

pub fn get(id: &str) -> Result<Account, String> {
    read_meta(id)
        .map(|meta| Account { id: id.to_string(), meta })
        .ok_or_else(|| format!("no saved account {id}"))
}

pub fn save(account: &Account) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&account.meta).map_err(|e| e.to_string())?;
    fs::write(account_dir(&account.id)?.join("meta.json"), json).map_err(|e| e.to_string())
}

// A not-yet-saved account: a folder without meta.json, until sign-in is
// detected. Named by time because Claude locks the folder while it runs, so
// it can't be renamed to the email afterwards.
pub fn create_pending() -> Result<String, String> {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let id = format!("acct-{ms}");
    fs::create_dir_all(user_data_dir(&id)?).map_err(|e| e.to_string())?;
    Ok(id)
}

// Only folders this app created count - never delete anything else that
// happens to lack a meta.json (old switcher backups, user folders).
pub fn is_pending(id: &str) -> bool {
    id.starts_with("acct-") && account_dir(id).is_ok_and(|d| d.is_dir() && !d.join("meta.json").exists())
}

// Left behind when the app closed in the middle of adding an account.
pub fn list_pending() -> Vec<String> {
    fs::read_dir(accounts_dir())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|id| is_pending(id))
        .collect()
}
