mod claude;
mod quota;
mod store;
mod token;

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountView {
    id: String,
    email: String,
    display_name: Option<String>,
    hotkey: Option<String>,
    color: Option<String>,
    running: bool,
}

// Swatches the UI offers, with the emoji that stands for them in the tray menu.
const COLORS: [(&str, &str); 6] =
    [("red", "🔴"), ("orange", "🟠"), ("yellow", "🟡"), ("green", "🟢"), ("blue", "🔵"), ("purple", "🟣")];

fn name(a: &store::Account) -> String {
    a.meta.display_name.clone().unwrap_or_else(|| a.meta.email.clone())
}

// The account the menu-bar title follows: the last one opened from here.
static CURRENT: Mutex<Option<String>> = Mutex::new(None);

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
            color: a.meta.color,
        })
        .collect()
}

fn open(id: &str) -> Result<(), String> {
    let dir = store::user_data_dir(id)?;
    // A failed merge only means fewer shared sessions - open the account
    // anyway, then report it.
    let synced = if claude::is_running(&claude::snapshot(), &dir) { Ok(()) } else { claude::sync_shared_sessions(&dir) };
    claude::launch(&dir)?;
    *CURRENT.lock().unwrap() = Some(id.to_string());
    synced
}

// Opens the account with the most quota left; returns its name.
fn open_best_account() -> Result<String, String> {
    let sys = claude::snapshot();
    let accounts = store::list();
    let cache = quota::CACHE.lock().unwrap().clone();
    let best = quota::pick_best(
        accounts.iter().map(|a| (a.id.as_str(), running(&sys, &a.id), cache.get(&a.id).and_then(|u| u.binding()))),
    )
    .ok_or("No account has usage data yet - open one first")?;
    open(best)?;
    Ok(accounts.iter().find(|a| a.id == best).map(name).unwrap_or_default())
}

#[tauri::command]
async fn open_best(app: AppHandle) -> Result<String, String> {
    let picked = open_best_account();
    update_tray_soon(&app);
    picked
}

#[tauri::command]
async fn open_account(app: AppHandle, id: String) -> Result<(), String> {
    store::get(&id)?;
    let opened = open(&id);
    update_tray_soon(&app);
    opened
}

#[tauri::command]
async fn stop_account(app: AppHandle, id: String) -> Result<(), String> {
    claude::stop(&store::user_data_dir(&id)?);
    update_tray_soon(&app);
    Ok(())
}

#[tauri::command]
fn usage_all() -> HashMap<String, quota::UsageState> {
    quota::CACHE.lock().unwrap().clone()
}

#[tauri::command]
async fn refresh_usage(app: AppHandle, id: Option<String>) -> HashMap<String, quota::UsageState> {
    refresh(&app, id.as_deref()).await
}

// Refetches usage (every account, or one), notifies limit crossings and pushes
// the result to the menu bar and the window ("usage" event).
async fn refresh(app: &AppHandle, only: Option<&str>) -> HashMap<String, quota::UsageState> {
    let accounts = store::list();
    let tasks: Vec<_> = accounts
        .iter()
        .filter(|a| only.is_none_or(|o| o == a.id))
        .map(|a| {
            let id = a.id.clone();
            tauri::async_runtime::spawn(async move {
                let state = quota::fetch(&id).await;
                (id, state)
            })
        })
        .collect();
    for task in tasks {
        let Ok((id, state)) = task.await else { continue };
        if let Some(a) = accounts.iter().find(|a| a.id == id) {
            notify_crossings(app, &name(a), &id, &state);
        }
        quota::CACHE.lock().unwrap().insert(id, state);
    }
    let map = {
        let mut cache = quota::CACHE.lock().unwrap();
        cache.retain(|id, _| accounts.iter().any(|a| &a.id == id));
        cache.clone()
    };
    let _ = app.emit("usage", &map);
    update_tray(app);
    map
}

fn notify_crossings(app: &AppHandle, name: &str, id: &str, state: &quota::UsageState) {
    let quota::UsageState::Ok { usage } = state else { return };
    for (key, label) in quota::WINDOWS {
        let Some(pct) = usage[key]["utilization"].as_f64() else { continue };
        let was = quota::LEVELS.lock().unwrap().insert((id.to_string(), key), pct >= quota::NEAR_LIMIT);
        let body = match quota::crossing(was, pct) {
            Some(true) => format!("{label} limit {pct:.0}% used"),
            Some(false) => format!("{label} limit reset - usable again"),
            None => continue,
        };
        let _ = app.notification().builder().title(name).body(body).show();
    }
}

// Tray: title = the current account's binding %, menu = every account (check =
// running) + open best. The menu is only replaced when its text changes, so an
// open menu isn't swapped out under the pointer every tick.
static TRAY_MENU: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn update_tray(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let sys = claude::snapshot();
    let accounts = store::list();
    let cache = quota::CACHE.lock().unwrap().clone();
    let rows: Vec<_> = accounts.iter().map(|a| (a, running(&sys, &a.id), cache.get(&a.id))).collect();

    let last = CURRENT.lock().unwrap().clone().filter(|id| rows.iter().any(|(a, on, _)| *on && &a.id == id));
    let current = last.or_else(|| {
        let mut on = rows.iter().filter(|(_, on, _)| *on);
        match (on.next(), on.next()) {
            (Some((a, _, _)), None) => Some(a.id.clone()),
            _ => None,
        }
    });
    let current = current.and_then(|id| rows.iter().find(|(a, _, _)| a.id == id));
    let _ = tray.set_title(current.and_then(|(_, _, u)| u.and_then(|u| u.binding())).map(|p| format!("{p:.0}%")));
    let _ = tray.set_tooltip(Some(current.map_or("Claude Accounts".into(), |(a, _, u)| {
        format!("{} - {}", name(a), u.map_or("usage loading".into(), |u| u.short()))
    })));

    let entries: Vec<(String, String, bool)> = rows
        .iter()
        .map(|(a, on, u)| {
            let dot = COLORS.iter().find(|(c, _)| a.meta.color.as_deref() == Some(*c)).map_or("", |(_, e)| *e);
            let text = format!("{dot} {}   {}", name(a), u.map_or("…".into(), |u| u.short()));
            (format!("acct:{}", a.id), text.trim_start().to_string(), *on)
        })
        .collect();
    let best = quota::pick_best(rows.iter().map(|(a, on, u)| (a.id.as_str(), *on, u.and_then(|u| u.binding()))))
        .and_then(|id| rows.iter().find(|(a, _, _)| a.id == id))
        .map(|(a, _, u)| format!("Open best: {} ({:.0}%)", name(a), u.and_then(|u| u.binding()).unwrap_or(0.0)));

    let sig: Vec<String> = entries.iter().map(|(id, t, on)| format!("{id}{t}{on}")).chain(best.clone()).collect();
    // Never hold the lock across set_menu: that waits for the main thread.
    if std::mem::replace(&mut *TRAY_MENU.lock().unwrap(), sig.clone()) == sig {
        return;
    }
    match tray_menu(app, &entries, best.as_deref()) {
        Ok(menu) => drop(tray.set_menu(Some(menu))),
        Err(_) => TRAY_MENU.lock().unwrap().clear(),
    }
}

fn tray_menu(app: &AppHandle, entries: &[(String, String, bool)], best: Option<&str>) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    for (id, text, on) in entries {
        menu.append(&CheckMenuItem::with_id(app, id, text, true, *on, None::<&str>)?)?;
    }
    if !entries.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "best", best.unwrap_or("Open best account"), best.is_some(), None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "show", "Show Claude Accounts", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?)?;
    Ok(menu)
}

// Claude takes a moment to start or quit - look again once it has.
fn update_tray_soon(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        update_tray(&app);
    });
}

#[tauri::command]
async fn rename_account(app: AppHandle, id: String, name: Option<String>) -> Result<(), String> {
    let mut account = store::get(&id)?;
    account.meta.display_name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    store::save(&account)?;
    update_tray(&app);
    Ok(())
}

#[tauri::command]
async fn set_color(app: AppHandle, id: String, color: Option<String>) -> Result<(), String> {
    if color.as_deref().is_some_and(|c| !COLORS.iter().any(|(n, _)| *n == c)) {
        return Err(format!("unknown colour {color:?}"));
    }
    let mut account = store::get(&id)?;
    account.meta.color = color;
    store::save(&account)?;
    update_tray(&app);
    Ok(())
}

// The whole list in its new order (drag and drop / Move to top).
#[tauri::command]
async fn set_order(app: AppHandle, ids: Vec<String>) -> Result<(), String> {
    for (i, id) in ids.iter().enumerate() {
        let mut account = store::get(id)?;
        account.meta.order = Some(i as i64);
        store::save(&account)?;
    }
    update_tray(&app);
    Ok(())
}

#[tauri::command]
fn best_hotkey() -> Option<String> {
    store::settings().best_hotkey
}

#[tauri::command]
async fn set_best_hotkey(app: AppHandle, hotkey: Option<String>) -> Result<(), String> {
    if let Some(h) = &hotkey {
        h.parse::<Shortcut>().map_err(|e| format!("{h}: {e}"))?;
        if let Some(a) = store::list().into_iter().find(|a| a.meta.hotkey.as_deref() == Some(h)) {
            return Err(format!("{h} is already used by {}", name(&a)));
        }
    }
    store::save_settings(&store::Settings { best_hotkey: hotkey })?;
    register_hotkeys(&app)
}

#[tauri::command]
fn set_hotkey(app: AppHandle, id: String, hotkey: Option<String>) -> Result<(), String> {
    if let Some(h) = &hotkey {
        h.parse::<Shortcut>().map_err(|e| format!("{h}: {e}"))?;
        if store::list().iter().any(|a| a.id != id && a.meta.hotkey.as_deref() == Some(h)) {
            return Err(format!("{h} is already used by another account"));
        }
        if store::settings().best_hotkey.as_deref() == Some(h) {
            return Err(format!("{h} is already used by Open best account"));
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
    let dir = store::user_data_dir(&id)?;
    if token::access_token(&dir).is_err() {
        return Ok(None);
    }
    // Signed in: a failed lookup (offline right after login) is an error, not
    // "not signed in" - that answer gets a leftover add's folder deleted.
    let email = token::email(&dir).await?;
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
    for h in store::list().into_iter().filter_map(|a| a.meta.hotkey).chain(store::settings().best_hotkey) {
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

fn on_hotkey(app: &AppHandle, shortcut: &Shortcut) {
    let is = |h: Option<&str>| h.and_then(|h| h.parse::<Shortcut>().ok()).as_ref() == Some(shortcut);
    if is(store::settings().best_hotkey.as_deref()) {
        if let Err(e) = open_best_account() {
            let _ = app.notification().builder().title("Claude Accounts").body(e).show();
        }
    } else if let Some(a) = store::list().into_iter().find(|a| is(a.meta.hotkey.as_deref())) {
        let _ = open(&a.id);
    }
    update_tray_soon(app);
}

// macOS steals key focus back once right after the popover is shown (blur fires
// immediately); a blur this soon after showing is refocused instead of hiding.
#[cfg(target_os = "macos")]
static SHOWN_AT: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        {
            place_popover(&w);
            *SHOWN_AT.lock().unwrap() = Some(std::time::Instant::now());
        }
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

// macOS: the window is a menu-bar popover - just under the menu bar, centred
// on the cursor (which is on the tray icon when it was clicked), kept on screen.
// The tray icon's own rect is useless here: macOS reports the whole menu bar
// (and (0,0) right after launch).
#[cfg(target_os = "macos")]
fn place_popover(w: &tauri::WebviewWindow) {
    let (Ok(cursor), Ok(size)) = (w.cursor_position(), w.outer_size()) else { return };
    let Ok(Some(m)) = w.monitor_from_point(cursor.x, cursor.y).map(|m| m.or(w.current_monitor().ok().flatten())) else { return };
    let area = m.work_area();
    let (left, width) = (area.position.x as f64, area.size.width as f64);
    let x = (cursor.x - size.width as f64 / 2.0).clamp(left + 8.0, left + width - size.width as f64 - 8.0);
    let y = area.position.y as f64 + 6.0 * m.scale_factor();
    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
}

// Left click on the tray: macOS toggles the popover, Windows just shows the window.
fn on_tray_click(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
            return;
        }
    }
    show_main(app);
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
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
    } else if app.autolaunch().is_enabled().unwrap_or(false) {
        // Re-point it at this exe: after a reinstall elsewhere, login would
        // start the old copy (or nothing).
        let _ = app.autolaunch().enable();
    }

    // Menu-bar-only on macOS: tao sets the Regular policy (Dock icon) at launch,
    // overriding LSUIElement, so ask for Accessory explicitly.
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().ok_or("no app icon")?)
        .tooltip("Claude Accounts")
        .menu(&tray_menu(app.handle(), &[], None)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| {
            match e.id.as_ref() {
                "show" => show_main(app),
                "quit" => app.exit(0),
                "best" => {
                    if let Err(e) = open_best_account() {
                        let _ = app.notification().builder().title("Claude Accounts").body(e).show();
                    }
                }
                id => {
                    if let Some(id) = id.strip_prefix("acct:") {
                        let _ = open(id);
                    }
                }
            }
            // A check item ticks itself when clicked - rebuild from the real state.
            TRAY_MENU.lock().unwrap().clear();
            update_tray_soon(app);
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                on_tray_click(tray.app_handle());
            }
        })
        .build(app)?;

    // Report the popover as a floating window (what NSPanel popovers report), not a
    // standard one: "quit when the last window closes" utilities (e.g. Vorssaint)
    // only track standard windows, so hiding the popover would otherwise quit the app.
    #[cfg(target_os = "macos")]
    if let Some(ns) = app.get_webview_window("main").and_then(|w| w.ns_window().ok()) {
        let role = objc2_foundation::NSString::from_str("AXFloatingWindow");
        unsafe {
            let ns = &*(ns as *const objc2::runtime::AnyObject);
            let _: () = objc2::msg_send![ns, setAccessibilitySubrole: &*role];
        }
    }

    // Running state for the tray every 30 s, usage every 5 min.
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        for tick in 0u64.. {
            if tick % 10 == 0 {
                tauri::async_runtime::block_on(refresh(&handle, None));
            } else {
                update_tray(&handle);
            }
            std::thread::sleep(Duration::from_secs(30));
        }
    });

    if !std::env::args().any(|a| a == "--minimized") {
        show_main(app.handle());
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app)))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        on_hotkey(app, shortcut);
                    }
                })
                .build(),
        )
        .setup(setup)
        .on_window_event(|w, e| match e {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = w.hide();
            }
            // The macOS popover closes when you click anywhere else.
            #[cfg(target_os = "macos")]
            WindowEvent::Focused(false) => {
                let just_shown = SHOWN_AT.lock().unwrap().take().is_some_and(|t| t.elapsed() < Duration::from_millis(500));
                let _ = if just_shown { w.set_focus() } else { w.hide() };
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            list_accounts,
            open_account,
            stop_account,
            usage_all,
            refresh_usage,
            open_best,
            set_color,
            set_order,
            best_hotkey,
            set_best_hotkey,
            rename_account,
            set_hotkey,
            add_start,
            add_poll,
            add_cancel,
            pending_accounts,
            autostart_enabled,
            set_autostart,
            quit
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
