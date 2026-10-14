mod claude;
mod store;
mod token;

use serde::Serialize;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountView {
    id: String,
    email: String,
    display_name: Option<String>,
    hotkey: Option<String>,
    running: bool,
}

fn running(sys: &sysinfo::System, id: &str) -> bool {
    store::user_data_dir(id).is_ok_and(|d| claude::is_running(sys, &d))
}

#[tauri::command]
async fn list_accounts() -> Vec<AccountView> {
    let sys = claude::snapshot();
    store::list()
        .into_iter()
        .map(|a| AccountView {
            running: running(&sys, &a.id),
            id: a.id,
            email: a.meta.email,
            display_name: a.meta.display_name,
            hotkey: a.meta.hotkey,
        })
        .collect()
}

fn open(id: &str) -> Result<(), String> {
    let dir = store::user_data_dir(id)?;
    if !claude::is_running(&claude::snapshot(), &dir) {
        claude::link_shared_sessions(&dir)?;
    }
    claude::launch(&dir)
}

#[tauri::command]
async fn open_account(id: String) -> Result<(), String> {
    store::get(&id)?;
    open(&id)
}

#[tauri::command]
async fn stop_account(id: String) -> Result<(), String> {
    claude::stop(&store::user_data_dir(&id)?);
    Ok(())
}

#[tauri::command]
async fn account_usage(id: String) -> Result<serde_json::Value, String> {
    token::usage(&store::user_data_dir(&id)?).await
}

#[tauri::command]
fn rename_account(id: String, name: Option<String>) -> Result<(), String> {
    let mut account = store::get(&id)?;
    account.meta.display_name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    store::save(&account)
}

#[tauri::command]
fn set_hotkey(app: AppHandle, id: String, hotkey: Option<String>) -> Result<(), String> {
    if let Some(h) = &hotkey {
        h.parse::<Shortcut>().map_err(|e| format!("{h}: {e}"))?;
        if store::list().iter().any(|a| a.id != id && a.meta.hotkey.as_deref() == Some(h)) {
            return Err(format!("{h} is already used by another account"));
        }
    }
    let mut account = store::get(&id)?;
    account.meta.hotkey = hotkey;
    store::save(&account)?;
    register_hotkeys(&app)
}

// Add account: a fresh, empty instance the user signs into once. Nothing
// else is closed or touched.
#[tauri::command]
async fn add_start() -> Result<String, String> {
    let id = store::create_pending()?;
    claude::launch(&store::user_data_dir(&id)?)?;
    Ok(id)
}

// Some(email) once the new instance is signed in and saved.
#[tauri::command]
async fn add_poll(id: String) -> Result<Option<String>, String> {
    if !store::is_pending(&id) {
        return Err("not a pending account".into());
    }
    let Ok(email) = token::email(&store::user_data_dir(&id)?).await else { return Ok(None) };
    if let Some(existing) = store::list().into_iter().find(|a| a.meta.email.eq_ignore_ascii_case(&email)) {
        let name = existing.meta.display_name.unwrap_or(existing.meta.email);
        return Err(format!("{email} is already saved ({name}) - cancel this one"));
    }
    store::save(&store::Account { id, meta: store::Meta { email: email.clone(), ..Default::default() } })?;
    Ok(Some(email))
}

// Unfinished adds as (id, still running).
#[tauri::command]
async fn pending_accounts() -> Vec<(String, bool)> {
    let sys = claude::snapshot();
    store::list_pending()
        .into_iter()
        .map(|id| {
            let is_running = running(&sys, &id);
            (id, is_running)
        })
        .collect()
}

#[tauri::command]
async fn add_cancel(id: String) -> Result<(), String> {
    // Only ever deletes a folder that was never saved as an account.
    if !store::is_pending(&id) {
        return Err("not a pending account".into());
    }
    let dir = store::user_data_dir(&id)?;
    // Waiting for Claude to let go of the folder blocks - keep it off the async runtime.
    tauri::async_runtime::spawn_blocking(move || {
        claude::stop(&dir);
        for _ in 0..20 {
            if !claude::is_running(&claude::snapshot(), &dir) {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    std::fs::remove_dir_all(store::account_dir(&id)?).map_err(|e| e.to_string())
}

fn register_hotkeys(app: &AppHandle) -> Result<(), String> {
    let gs = app.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;
    let mut failed = Vec::new();
    for a in store::list() {
        let Some(h) = a.meta.hotkey else { continue };
        if !h.parse::<Shortcut>().is_ok_and(|s| gs.register(s).is_ok()) {
            failed.push(h);
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("could not register {} (taken by another app?)", failed.join(", ")))
    }
}

fn on_hotkey(shortcut: &Shortcut) {
    let hit = store::list()
        .into_iter()
        .find(|a| a.meta.hotkey.as_deref().and_then(|h| h.parse::<Shortcut>().ok()).as_ref() == Some(shortcut));
    if let Some(a) = hit {
        let _ = open(&a.id);
    }
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn autostart_enabled(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    if on { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())
}

// Starts at login hidden in the tray (hotkeys need the app running); closing
// the window only hides it. Quit from the tray menu.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let _ = register_hotkeys(app.handle());

    // Start at login is on by default - set once, then left to the user's toggle.
    let decided = app.path().app_config_dir()?.join("autostart-decided");
    if !decided.exists() {
        let _ = app.autolaunch().enable();
        std::fs::create_dir_all(decided.parent().unwrap())?;
        std::fs::write(&decided, "")?;
    }

    let show = MenuItem::with_id(app, "show", "Show Claude Accounts", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().ok_or("no app icon")?)
        .tooltip("Claude Accounts")
        .menu(&Menu::with_items(app, &[&show, &quit])?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;

    if !std::env::args().any(|a| a == "--minimized") {
        show_main(app.handle());
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|_, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        on_hotkey(shortcut);
                    }
                })
                .build(),
        )
        .setup(setup)
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                let _ = w.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_accounts,
            open_account,
            stop_account,
            account_usage,
            rename_account,
            set_hotkey,
            add_start,
            add_poll,
            add_cancel,
            pending_accounts,
            autostart_enabled,
            set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running Claude Accounts");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_cannot_escape_store() {
        for bad in ["", ".", "..", "../x", "a/b", r"a\b"] {
            assert!(store::account_dir(bad).is_err(), "{bad}");
        }
        assert!(store::account_dir("sdp1@example.com").is_ok());
    }

    // Live check against this machine's saved accounts (nothing is changed):
    // which instances run, whether each token decrypts, and plan usage.
    // cargo test probe -- --ignored --nocapture
    #[test]
    #[ignore]
    fn probe() {
        tauri::async_runtime::block_on(async {
            for a in list_accounts().await {
                let dir = store::user_data_dir(&a.id).unwrap();
                let token = token::access_token(&dir).map(|t| format!("ok ({} chars)", t.len()));
                let usage = token::usage(&dir).await.map(|u| {
                    format!("5h {}% / 7d {}%", u["five_hour"]["utilization"], u["seven_day"]["utilization"])
                });
                println!("{} running={} token={:?} usage={:?}", a.email, a.running, token, usage);
            }
        });
    }
}
