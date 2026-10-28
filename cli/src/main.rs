// seatdeck-cli: save several Claude Code logins and switch between them.
// Behaviour is specified in ../SPEC.md.

use serde_json::{json, Value};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const KEEP_BACKUPS: usize = 5;
// Keys of the credentials JSON that belong to the account; the rest (mcpOAuth...)
// belongs to the machine and is never swapped.
const ACCOUNT_KEYS: [&str; 2] = ["claudeAiOauth", "organizationUuid"];

type Res<T> = Result<T, String>;

const HELP: &str = "seatdeck-cli - switch the account Claude Code runs under

  seatdeck-cli add [email] [--force]   save the account Claude Code is logged into
  seatdeck-cli list                    saved accounts with 5-hour / 7-day usage
  seatdeck-cli switch [number|email]   switch Claude Code to a saved account
  seatdeck-cli doctor                  check the setup

After switching, restart any open Claude Code session; `claude --resume`
continues the same conversation under the new account.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> Res<()> {
    let p = Paths::detect()?;
    match args.first().map(String::as_str) {
        Some("add") => add(&p, &args[1..]),
        Some("list" | "ls") => list(&p),
        Some("switch") => switch(&p, args.get(1).map(String::as_str)),
        Some("doctor") => doctor(&p),
        Some("-V" | "--version" | "version") => {
            println!("seatdeck-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        None | Some("-h" | "--help" | "help") => {
            println!("{HELP}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`\n\n{HELP}")),
    }
}

// ---------------------------------------------------------------- locations --

struct Paths {
    home: PathBuf,
    creds_file: PathBuf,
    global_cfg: PathBuf,
    store: PathBuf,
}

impl Paths {
    fn detect() -> Res<Paths> {
        #[allow(deprecated)]
        let home = std::env::home_dir().ok_or("cannot find the home folder")?;
        let custom = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()).map(PathBuf::from);
        Ok(Paths {
            creds_file: custom.clone().unwrap_or_else(|| home.join(".claude")).join(".credentials.json"),
            global_cfg: custom.unwrap_or_else(|| home.clone()).join(".claude.json"),
            store: home.join(".claude-cc-accounts"),
            home,
        })
    }
    fn backups(&self) -> PathBuf {
        self.store.join(".backups")
    }
    fn marker(&self) -> PathBuf {
        self.store.join(".current")
    }
}

// --------------------------------------------------------------- json files --

fn parse(text: &str) -> Res<Value> {
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())
}

fn read_json(path: &Path) -> Option<Value> {
    parse(&fs::read_to_string(path).ok()?).ok()
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).expect("a Value always serializes")
}

// Writes through a temp file + rename so a crash never leaves half a file.
fn write_private(path: &Path, text: &str) -> Res<()> {
    let err = |e: io::Error| format!("{}: {e}", path.display());
    let dir = path.parent().ok_or("bad path")?;
    fs::create_dir_all(dir).map_err(err)?;
    let tmp = dir.join(format!(".{}.seatdeck-tmp", path.file_name().unwrap_or_default().to_string_lossy()));
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
    opts.open(&tmp).and_then(|mut f| f.write_all(text.as_bytes())).map_err(err)?;
    fs::rename(&tmp, path).map_err(err)
}

fn ensure_store(p: &Paths) -> Res<()> {
    fs::create_dir_all(p.backups()).map_err(|e| format!("{}: {e}", p.store.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&p.store, fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn token(creds: &Value) -> Option<&str> {
    creds["claudeAiOauth"]["accessToken"].as_str()
}

fn email_of(identity: &Value) -> Option<&str> {
    identity["emailAddress"].as_str()
}

fn same_email(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

// --------------------------------------------------------- live credentials --

#[cfg(not(target_os = "macos"))]
fn read_live(p: &Paths) -> Option<Value> {
    read_json(&p.creds_file)
}

#[cfg(not(target_os = "macos"))]
fn write_live(p: &Paths, creds: &Value) -> Res<()> {
    write_private(&p.creds_file, &pretty(creds))
}

#[cfg(not(target_os = "macos"))]
fn delete_live(p: &Paths) -> Res<()> {
    fs::remove_file(&p.creds_file).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn read_live(_: &Paths) -> Option<Value> {
    let out = std::process::Command::new("security")
        .args(["find-generic-password", "-s", KEYCHAIN_SERVICE, "-w"])
        .output()
        .ok()?;
    out.status.success().then(|| parse(String::from_utf8_lossy(&out.stdout).trim()).ok()).flatten()
}

#[cfg(target_os = "macos")]
fn write_live(_: &Paths, creds: &Value) -> Res<()> {
    use std::process::{Command, Stdio};
    let user = std::env::var("USER").map_err(|_| "USER is not set")?;
    let hex: String = serde_json::to_string(creds).unwrap().bytes().map(|b| format!("{b:02x}")).collect();
    // Fed on stdin so the login never shows up in `ps`; `security -i` reads at
    // most ~4 KB per line, so a bigger blob falls back to an argument.
    let line = format!("add-generic-password -U -a \"{user}\" -s \"{KEYCHAIN_SERVICE}\" -X {hex}\n");
    let out = if line.len() < 4000 {
        let mut child = Command::new("security")
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("security: {e}"))?;
        child.stdin.take().unwrap().write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        child.wait_with_output()
    } else {
        Command::new("security").args(["add-generic-password", "-U", "-a", &user, "-s", KEYCHAIN_SERVICE, "-X", &hex]).output()
    }
    .map_err(|e| format!("security: {e}"))?;
    if out.status.success() && out.stderr.is_empty() {
        Ok(())
    } else {
        Err(format!("Keychain write failed: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

#[cfg(target_os = "macos")]
fn delete_live(_: &Paths) -> Res<()> {
    let ok = std::process::Command::new("security")
        .args(["delete-generic-password", "-s", KEYCHAIN_SERVICE])
        .output()
        .map_err(|e| e.to_string())?
        .status
        .success();
    if ok { Ok(()) } else { Err("Keychain delete failed".into()) }
}

fn live_label(p: &Paths) -> String {
    if cfg!(target_os = "macos") {
        format!("macOS Keychain, service \"{KEYCHAIN_SERVICE}\"")
    } else {
        p.creds_file.display().to_string()
    }
}

// ------------------------------------------------- identity in .claude.json --

fn live_identity(p: &Paths) -> Option<Value> {
    read_json(&p.global_cfg)?.get("oauthAccount").filter(|v| v.is_object()).cloned()
}

// Returns whether the file changed. The previous file is kept in .backups.
fn set_identity(p: &Paths, identity: &Value) -> Res<bool> {
    let Ok(text) = fs::read_to_string(&p.global_cfg) else { return Ok(false) };
    let mut cfg = parse(&text).map_err(|e| format!("{}: {e}", p.global_cfg.display()))?;
    if !cfg.is_object() || cfg["oauthAccount"] == *identity {
        return Ok(false);
    }
    ensure_store(p)?;
    write_private(&p.backups().join("claude.json.bak"), &text)?;
    cfg["oauthAccount"] = identity.clone();
    write_private(&p.global_cfg, &pretty(&cfg))?;
    Ok(true)
}

// ----------------------------------------------------------- saved accounts --

struct Account {
    file: PathBuf,
    email: String,
    doc: Value,
}

impl Account {
    fn creds(&self) -> &Value {
        &self.doc["credentials"]
    }
    fn token(&self) -> Option<&str> {
        token(self.creds())
    }
    // Stored as JSON text (oauthAccountRaw), the format earlier versions wrote.
    fn identity(&self) -> Option<Value> {
        self.doc["oauthAccountRaw"].as_str().and_then(|s| parse(s).ok()).filter(Value::is_object)
    }
}

fn file_name_for(email: &str) -> String {
    let safe: String = email
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || "_.@+-".contains(c) { c } else { '_' })
        .collect();
    format!("{safe}.json")
}

fn load_accounts(store: &Path) -> Vec<Account> {
    let mut files: Vec<PathBuf> = fs::read_dir(store)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    files.sort();
    files
        .into_iter()
        .filter(|f| {
            let name = f.file_name().unwrap_or_default().to_string_lossy();
            name.ends_with(".json") && !name.starts_with("backup-") && f.is_file()
        })
        .filter_map(|file| {
            let doc = read_json(&file)?;
            let email = doc["email"].as_str()?.to_string();
            doc["credentials"].is_object().then_some(Account { file, email, doc })
        })
        .collect()
}

fn save_account(file: &Path, email: &str, creds: &Value, identity: Option<&Value>) -> Res<()> {
    let mut doc = json!({ "email": email, "savedAt": fmt_time(now_secs()), "credentials": creds });
    if let Some(id) = identity {
        doc["oauthAccountRaw"] = Value::String(serde_json::to_string(id).unwrap());
    }
    write_private(file, &pretty(&doc))
}

// Which saved account the live login belongs to (see SPEC.md, "The current account").
fn find_current(accounts: &[Account], live: Option<&Value>, live_email: Option<&str>, marker: Option<&str>) -> Option<usize> {
    let tok = live.and_then(token)?;
    if let Some(i) = accounts.iter().position(|a| a.token() == Some(tok)) {
        return Some(i);
    }
    let email = live_email?;
    if marker.is_some_and(|m| !same_email(m, email)) {
        return None;
    }
    accounts.iter().position(|a| same_email(&a.email, email))
}

// The live credentials with the account part replaced by `saved`'s.
fn merge_login(live: Option<&Value>, saved: &Value) -> Value {
    let mut next = live.filter(|v| v.is_object()).cloned().unwrap_or_else(|| json!({}));
    let obj = next.as_object_mut().unwrap();
    for key in ACCOUNT_KEYS {
        match saved.get(key) {
            Some(v) => obj.insert(key.to_string(), v.clone()),
            None => obj.remove(key),
        };
    }
    next
}

struct State {
    accounts: Vec<Account>,
    live: Option<Value>,
    identity: Option<Value>,
    current: Option<usize>,
}

// Loads everything and writes a refreshed live login back into its saved file.
fn load(p: &Paths) -> Res<State> {
    let mut accounts = load_accounts(&p.store);
    let live = read_live(p);
    let identity = live_identity(p);
    let marker = fs::read_to_string(p.marker()).ok();
    let live_email = identity.as_ref().and_then(email_of);
    let current = find_current(&accounts, live.as_ref(), live_email, marker.as_deref());
    if let (Some(i), Some(live)) = (current, &live) {
        let a = &mut accounts[i];
        if a.token() != token(live) {
            let id = identity.as_ref().filter(|id| email_of(id).is_some_and(|e| same_email(e, &a.email))).cloned().or_else(|| a.identity());
            save_account(&a.file, &a.email, live, id.as_ref())?;
            a.doc["credentials"] = live.clone();
            println!("(refreshed the saved login of {})", a.email);
        }
    }
    Ok(State { accounts, live, identity, current })
}

fn backup_live(p: &Paths, live: &Value) -> Res<PathBuf> {
    ensure_store(p)?;
    let stamp: String = fmt_time(now_secs()).chars().filter(char::is_ascii_digit).collect();
    let file = p.backups().join(format!("backup-{stamp}.credentials.json"));
    write_private(&file, &pretty(live))?;
    let mut old: Vec<PathBuf> = fs::read_dir(p.backups())
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    old.retain(|f| f.file_name().unwrap_or_default().to_string_lossy().starts_with("backup-"));
    old.sort();
    old.reverse();
    for f in old.iter().skip(KEEP_BACKUPS) {
        let _ = fs::remove_file(f);
    }
    Ok(file)
}

// -------------------------------------------------------------------- usage --

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn fetch_usage(creds: &Value) -> Res<Value> {
    let oauth = &creds["claudeAiOauth"];
    let tok = oauth["accessToken"].as_str().ok_or("no token saved")?;
    let expired = "login expired - switch to it and run `claude` once, or /login again";
    if oauth["expiresAt"].as_i64().is_some_and(|ms| ms > 0 && ms / 1000 < now_secs()) {
        return Err(expired.into());
    }
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(8)).build();
    let resp = agent
        .get(USAGE_URL)
        .set("Authorization", &format!("Bearer {tok}"))
        .set("anthropic-version", "2023-06-01")
        .set("anthropic-beta", "oauth-2025-04-20")
        .call();
    match resp {
        Ok(r) => r.into_json().map_err(|e| e.to_string()),
        Err(ureq::Error::Status(401, _)) => Err(expired.into()),
        Err(ureq::Error::Status(code, _)) => Err(format!("api.anthropic.com returned {code}")),
        Err(_) => Err("could not reach api.anthropic.com".into()),
    }
}

fn fmt_usage(u: &Value, now: i64) -> String {
    let window = |key: &str, label: &str| {
        let w = &u[key];
        let Some(pct) = w["utilization"].as_f64() else { return format!("{label}   -") };
        let reset = w["resets_at"].as_str().and_then(parse_time).map(|t| t - now).filter(|d| *d > 0);
        format!("{label} {pct:>3.0}%{}", reset.map(|d| format!(" (resets in {})", fmt_dur(d))).unwrap_or_default())
    };
    format!("{}   {}", window("five_hour", "5h"), window("seven_day", "7d"))
}

fn print_list(st: &State) {
    let usage: Vec<Res<Value>> = std::thread::scope(|s| {
        let jobs: Vec<_> = st.accounts.iter().map(|a| s.spawn(move || fetch_usage(a.creds()))).collect();
        jobs.into_iter().map(|j| j.join().unwrap_or_else(|_| Err("failed".into()))).collect()
    });
    let width = st.accounts.iter().map(|a| a.email.len()).max().unwrap_or(0);
    let now = now_secs();
    for (i, (a, u)) in st.accounts.iter().zip(&usage).enumerate() {
        let mark = if st.current == Some(i) { '*' } else { ' ' };
        let usage = match u {
            Ok(u) => fmt_usage(u, now),
            Err(e) => format!("usage n/a ({e})"),
        };
        println!("  {:>2} {mark} {:<width$}   {usage}", i + 1, a.email);
    }
    if st.current.is_none() {
        println!("  (the live login is not one of these - `seatdeck-cli add` saves it)");
    }
}

// ----------------------------------------------------------------- commands --

fn prompt(question: &str) -> Res<String> {
    print!("{question}");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
    Ok(line.trim().to_string())
}

fn add(p: &Paths, args: &[String]) -> Res<()> {
    let force = args.iter().any(|a| a == "--force");
    let given = args.iter().find(|a| !a.starts_with("--")).cloned();
    let live = read_live(p).ok_or("Claude Code is not logged in here - run `claude`, use /login, then try again")?;
    if !live["claudeAiOauth"].is_object() {
        return Err("the live login is not a Claude subscription login (API key setup?)".into());
    }
    let identity = live_identity(p);
    let detected = identity.as_ref().and_then(email_of).map(str::to_string);
    let email = match (given, &detected) {
        (Some(e), _) => e,
        (None, Some(d)) => d.clone(),
        (None, None) => prompt("Email of the account logged into Claude Code: ")?,
    };
    if email.is_empty() {
        return Err("cancelled".into());
    }
    if let Some(d) = detected.as_deref().filter(|d| !same_email(d, &email)) {
        if !force {
            return Err(format!(
                "Claude Code is logged in as {d}, not {email}; saving would file {d}'s login under {email}.\n\
                 Use /login as {email} first, or pass --force."
            ));
        }
    }
    let accounts = load_accounts(&p.store);
    if let Some(other) = accounts.iter().find(|a| a.token() == token(&live) && !same_email(&a.email, &email)) {
        if !force {
            return Err(format!("this login is already saved as {} (pass --force to save it again)", other.email));
        }
    }
    let identity = identity.filter(|id| email_of(id).is_some_and(|e| same_email(e, &email)));
    ensure_store(p)?;
    let existing = accounts.iter().find(|a| same_email(&a.email, &email)).map(|a| a.file.clone());
    let file = existing.clone().unwrap_or_else(|| p.store.join(file_name_for(&email)));
    save_account(&file, email.trim(), &live, identity.as_ref())?;
    write_private(&p.marker(), email.trim())?;
    println!("{} {}.", if existing.is_some() { "Updated" } else { "Saved" }, email.trim());

    if io::stdin().is_terminal() {
        let answer = prompt("Clear the local login now, so you can /login as another account? Nothing is revoked. [y/N] ")?;
        if answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
            let mut rest = live.clone();
            rest.as_object_mut().unwrap().retain(|k, _| !ACCOUNT_KEYS.contains(&k.as_str()));
            if rest.as_object().unwrap().is_empty() { delete_live(p)? } else { write_live(p, &rest)? }
            println!("Cleared. In Claude Code use /login as the next account, then run `seatdeck-cli add` again.");
            println!("(Restart any open Claude Code session first.)");
        }
    }
    Ok(())
}

fn list(p: &Paths) -> Res<()> {
    let st = load(p)?;
    if st.accounts.is_empty() {
        println!("No saved accounts. Log into Claude Code (/login), then run `seatdeck-cli add`.");
        return Ok(());
    }
    print_list(&st);
    Ok(())
}

fn pick(accounts: &[Account], choice: &str) -> Res<usize> {
    match choice.parse::<usize>() {
        Ok(n) if (1..=accounts.len()).contains(&n) => Ok(n - 1),
        Ok(_) => Err(format!("pick a number from 1 to {}", accounts.len())),
        Err(_) => accounts
            .iter()
            .position(|a| same_email(&a.email, choice))
            .ok_or_else(|| format!("no saved account {choice}")),
    }
}

fn switch(p: &Paths, choice: Option<&str>) -> Res<()> {
    let st = load(p)?;
    if st.accounts.is_empty() {
        return Err("no saved accounts - log into Claude Code (/login), then run `seatdeck-cli add`".into());
    }
    let i = match choice {
        Some(c) => pick(&st.accounts, c)?,
        None => {
            print_list(&st);
            let answer = prompt("Switch to (number, Enter = cancel): ")?;
            if answer.is_empty() {
                println!("Cancelled.");
                return Ok(());
            }
            pick(&st.accounts, &answer)?
        }
    };
    let a = &st.accounts[i];
    if st.current == Some(i) {
        if a.identity().map(|id| set_identity(p, &id)).transpose()? == Some(true) {
            println!("Fixed the account shown by /status.");
        }
        println!("Already on {}.", a.email);
        return Ok(());
    }

    let backup = st.live.as_ref().map(|l| backup_live(p, l)).transpose()?;
    write_live(p, &merge_login(st.live.as_ref(), a.creds()))?;
    if read_live(p).as_ref().and_then(token) != a.token() {
        let hint = backup.map(|b| format!(" - the previous login is in {}", b.display())).unwrap_or_default();
        return Err(format!("the new login did not stick{hint}"));
    }
    match a.identity() {
        Some(id) => {
            set_identity(p, &id)?;
        }
        None if st.identity.is_some() => println!(
            "note: no identity saved for {} - /status may show the previous email until you run `seatdeck-cli add` once while on it.",
            a.email
        ),
        None => {}
    }
    write_private(&p.marker(), &a.email)?;
    println!("Switched Claude Code to {}.", a.email);
    println!("Restart any open Claude Code session, then `claude --resume` to carry on.");
    Ok(())
}

fn doctor(p: &Paths) -> Res<()> {
    let line = |k: &str, v: &str| println!("  {k:<20}{v}");
    let mut problems = Vec::new();
    line("Platform", &format!("{} {}", std::env::consts::OS, std::env::consts::ARCH));
    line("Version", env!("CARGO_PKG_VERSION"));
    line("Login stored in", &live_label(p));
    line("Identity file", &p.global_cfg.display().to_string());
    if cfg!(target_os = "macos") {
        if std::process::Command::new("security").arg("-h").output().is_err() {
            problems.push("the `security` command is missing - the Keychain cannot be read".to_string());
        }
        if std::env::var_os("CLAUDE_CONFIG_DIR").is_some() {
            problems.push("CLAUDE_CONFIG_DIR is set: Claude Code then uses a different Keychain item, which this tool does not handle".to_string());
        }
    }

    let st = load(p)?;
    let live_email = st.identity.as_ref().and_then(email_of).unwrap_or("unknown email");
    match &st.live {
        Some(l) if l["claudeAiOauth"].is_object() => line("Live login", &format!("yes ({live_email})")),
        Some(_) => line("Live login", "no Claude subscription login (API key setup?)"),
        None => line("Live login", "none - run `claude`, use /login"),
    }
    line("Saved accounts", &format!("{} in {}", st.accounts.len(), p.store.display()));
    for (i, a) in st.accounts.iter().enumerate() {
        let tag = if st.current == Some(i) { "   <- current" } else { "" };
        let id = if a.identity().is_some() { "" } else { "   (no identity saved)" };
        println!("      - {}{tag}{id}", a.email);
    }
    if st.live.is_some() && st.current.is_none() {
        problems.push("the live login is not saved yet - run `seatdeck-cli add`".to_string());
    }
    if p.home.join(".claude-tools").join("claude-code-switch.ps1").exists() {
        problems.push("the old PowerShell switcher is still installed in ~/.claude-tools - rerun the installer to remove it".to_string());
    }

    println!();
    if problems.is_empty() && !st.accounts.is_empty() {
        println!("Looks good - `seatdeck-cli switch` is ready.");
    } else if problems.is_empty() {
        println!("No accounts saved yet - /login in Claude Code, then `seatdeck-cli add`.");
    }
    for pr in problems {
        println!("! {pr}");
    }
    Ok(())
}

// --------------------------------------------------------------------- time --

// Days since 1970-01-01 for a proleptic Gregorian date, and back.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

// RFC 3339, e.g. 2026-10-28T20:00:00.123+07:00 -> Unix seconds.
fn parse_time(s: &str) -> Option<i64> {
    let n = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let secs = days_from_civil(n(0, 4)?, n(5, 7)?, n(8, 10)?) * 86_400 + n(11, 13)? * 3600 + n(14, 16)? * 60 + n(17, 19)?;
    let zone = s.get(19..)?.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match zone {
        "" | "Z" | "z" => 0,
        z => {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            sign * (z.get(1..3)?.parse::<i64>().ok()? * 3600 + z.get(4..6)?.parse::<i64>().ok()? * 60)
        }
    };
    Some(secs - offset)
}

fn fmt_time(secs: i64) -> String {
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let t = secs.rem_euclid(86_400);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", t / 3600, t / 60 % 60, t % 60)
}

fn fmt_dur(secs: i64) -> String {
    match secs {
        s if s >= 86_400 => format!("{}d{}h", s / 86_400, s % 86_400 / 3600),
        s if s >= 3600 => format!("{}h{:02}m", s / 3600, s % 3600 / 60),
        s => format!("{}m", (s + 59) / 60),
    }
}

// -------------------------------------------------------------------- tests --

#[cfg(test)]
mod tests {
    use super::*;

    fn acct(email: &str, tok: &str) -> Account {
        Account {
            file: PathBuf::from(file_name_for(email)),
            email: email.into(),
            doc: json!({ "email": email, "credentials": { "claudeAiOauth": { "accessToken": tok } } }),
        }
    }

    fn sandbox(name: &str) -> Paths {
        let home = std::env::temp_dir().join(format!("seatdeck-cli-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(home.join(".claude")).unwrap();
        Paths {
            creds_file: home.join(".claude/.credentials.json"),
            global_cfg: home.join(".claude.json"),
            store: home.join(".claude-cc-accounts"),
            home,
        }
    }

    #[test]
    fn current_by_token_then_guarded_email() {
        let accts = [acct("a@x.com", "ta"), acct("b@x.com", "tb")];
        let live = |t: &str| json!({ "claudeAiOauth": { "accessToken": t } });
        assert_eq!(find_current(&accts, Some(&live("tb")), Some("a@x.com"), None), Some(1));
        // Refreshed token: fall back to the email Claude Code shows...
        assert_eq!(find_current(&accts, Some(&live("new")), Some("A@x.com"), None), Some(0));
        assert_eq!(find_current(&accts, Some(&live("new")), Some("a@x.com"), Some("a@x.com\n")), Some(0));
        // ...unless the last switch went elsewhere (identity may be stale).
        assert_eq!(find_current(&accts, Some(&live("new")), Some("a@x.com"), Some("b@x.com")), None);
        assert_eq!(find_current(&accts, None, Some("a@x.com"), None), None);
        assert_eq!(find_current(&accts, Some(&live("new")), Some("c@x.com"), None), None);
    }

    #[test]
    fn merge_keeps_machine_keys() {
        let live = json!({ "claudeAiOauth": { "accessToken": "old" }, "organizationUuid": "o1", "mcpOAuth": { "srv": 1 } });
        let saved = json!({ "claudeAiOauth": { "accessToken": "new" }, "mcpOAuth": { "stale": 1 } });
        let next = merge_login(Some(&live), &saved);
        assert_eq!(next, json!({ "claudeAiOauth": { "accessToken": "new" }, "mcpOAuth": { "srv": 1 } }));
        assert_eq!(merge_login(None, &saved)["claudeAiOauth"]["accessToken"], "new");
    }

    #[test]
    fn time_round_trip() {
        assert_eq!(parse_time("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_time("2026-10-28T20:00:00.123456+07:00"), parse_time("2026-10-28T13:00:00Z"));
        assert_eq!(parse_time("2024-02-29T12:00:00-01:30"), parse_time("2024-02-29T13:30:00Z"));
        assert_eq!(parse_time("nonsense"), None);
        for t in [0, 951_782_400, 1_793_192_400, 4_102_444_799] {
            assert_eq!(parse_time(&fmt_time(t)), Some(t));
        }
        assert_eq!(fmt_time(1_709_208_000), "2024-02-29T12:00:00Z");
        assert_eq!((fmt_dur(90_000), fmt_dur(7_800), fmt_dur(61)), ("1d1h".into(), "2h10m".into(), "2m".into()));
    }

    #[test]
    fn usage_line() {
        let u = json!({ "five_hour": { "utilization": 12.4, "resets_at": "1970-01-01T02:10:00Z" }, "seven_day": { "utilization": 40.0, "resets_at": null } });
        assert_eq!(fmt_usage(&u, 0), "5h  12% (resets in 2h10m)   7d  40%");
        assert_eq!(fmt_usage(&json!({}), 0), "5h   -   7d   -");
    }

    #[test]
    fn file_names() {
        assert_eq!(file_name_for(" a.b+c@x.com "), "a.b+c@x.com.json");
        assert_eq!(file_name_for("a/b\\c:d"), "a_b_c_d.json");
    }

    #[test]
    fn reads_accounts_saved_by_the_powershell_version() {
        let p = sandbox("ps-compat");
        fs::create_dir_all(p.backups()).unwrap();
        let old = "\u{feff}{\r\n    \"email\":  \"a@x.com\",\r\n    \"savedAt\":  \"2026-09-01T10:00:00.0000000+07:00\",\r\n    \"credentials\":  {\r\n        \"claudeAiOauth\":  { \"accessToken\":  \"ta\" }\r\n    },\r\n    \"oauthAccountRaw\":  \"{\\n  \\\"emailAddress\\\": \\\"a@x.com\\\"\\n}\"\r\n}";
        fs::write(p.store.join("a@x.com.json"), old).unwrap();
        fs::write(p.store.join("backup-1.json"), old).unwrap();
        fs::write(p.store.join("notes.json"), "{}").unwrap();
        let accts = load_accounts(&p.store);
        assert_eq!(accts.len(), 1);
        assert_eq!(accts[0].token(), Some("ta"));
        assert_eq!(accts[0].identity(), Some(json!({ "emailAddress": "a@x.com" })));
    }

    #[test]
    fn identity_edit_keeps_the_rest_of_claude_json() {
        let p = sandbox("identity");
        let original = "{\n  \"numStartups\": 12,\n  \"big\": 12345678901234567890.50,\n  \"oauthAccount\": {\n    \"emailAddress\": \"a@x.com\"\n  },\n  \"zLast\": []\n}";
        fs::write(&p.global_cfg, original).unwrap();
        assert!(set_identity(&p, &json!({ "emailAddress": "b@x.com" })).unwrap());
        let now = fs::read_to_string(&p.global_cfg).unwrap();
        assert_eq!(now, original.replace("a@x.com", "b@x.com"));
        assert_eq!(fs::read_to_string(p.backups().join("claude.json.bak")).unwrap(), original);
        assert!(!set_identity(&p, &json!({ "emailAddress": "b@x.com" })).unwrap());
    }

    #[test]
    fn sync_back_and_backups() {
        let p = sandbox("sync");
        ensure_store(&p).unwrap();
        save_account(&p.store.join("a@x.com.json"), "a@x.com", &json!({ "claudeAiOauth": { "accessToken": "old" } }), None).unwrap();
        fs::write(&p.global_cfg, r#"{"oauthAccount":{"emailAddress":"a@x.com"}}"#).unwrap();
        let live = json!({ "claudeAiOauth": { "accessToken": "rotated" }, "mcpOAuth": {} });
        write_private(&p.creds_file, &pretty(&live)).unwrap();
        // Only exercises the file backend; macOS reads the Keychain instead.
        if cfg!(not(target_os = "macos")) {
            let st = load(&p).unwrap();
            assert_eq!(st.current, Some(0));
            let saved = &load_accounts(&p.store)[0];
            assert_eq!(saved.token(), Some("rotated"));
            assert_eq!(saved.identity(), Some(json!({ "emailAddress": "a@x.com" })));
        }
        for n in 0..KEEP_BACKUPS + 2 {
            fs::write(p.backups().join(format!("backup-2000010100000{n}.credentials.json")), "{}").unwrap();
        }
        let newest = backup_live(&p, &live).unwrap();
        assert_eq!(read_json(&newest), Some(live));
        let kept = fs::read_dir(p.backups()).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().starts_with("backup-")).count();
        assert_eq!(kept, KEEP_BACKUPS);
    }
}
