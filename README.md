# Claude Account Switcher

Use several of your own **Anthropic Claude** accounts (e.g. a few Pro/Max
seats) without ever logging out and back in.

- **Claude Accounts** app *(Windows · macOS)* — keeps every Claude Desktop
  account signed in, each in its own window. Click (or press a hotkey) to open
  one; see each account's 5-hour / 7-day usage at a glance; Claude Code
  sessions from every account show up in every account.
- **Claude Code CLI switcher** *(Windows · macOS · Linux)* — swaps which
  account `claude` runs under, so you can hit a limit, switch, and
  `claude --resume` the same work.

| | Windows | macOS | Linux |
|---|:---:|:---:|:---:|
| Claude Accounts app (Claude Desktop) | ✅ | ⚠️ built, not yet verified | — |
| Claude Code CLI switcher | ✅ | ✅ | ✅ |

---

## Claude Accounts (Claude Desktop)

### Install

Download the installer from the repository's **Releases** page:

- **Windows** — `Claude Accounts_<version>_x64-setup.exe` (installs for your
  user only, no admin). The build is not code-signed: if SmartScreen says
  "Windows protected your PC", click **More info → Run anyway**.
- **macOS** — `Claude Accounts_<version>_universal.dmg`, drag the app to
  Applications. Not notarized: the first time, right-click the app → **Open**
  (or run `xattr -dr com.apple.quarantine "/Applications/Claude Accounts.app"`).

Claude Desktop itself must be installed (Microsoft Store / claude.ai download on
Windows, `/Applications/Claude.app` on macOS).

### Use

- **Add account** — opens a fresh, empty Claude window. Sign in there once; the
  app notices and saves the account. Nothing else is closed. Repeat per account.
- **Click an account** to open it. If it is already open, its window comes to
  the front. The dot is green while it runs.
- **⋯ menu** (or right-click) — Stop, Rename, Set / remove a global hotkey,
  Refresh usage.
- **Usage bars** — 5-hour and 7-day plan usage with reset times, refreshed every
  5 minutes (amber from 80 %, red from 95 %). An account that has been closed
  for a long time may show "expired" until you open it once.

- **Tray** — closing the window keeps the app running in the tray (menu bar on
  macOS) so hotkeys keep working; **Quit** is in the tray menu. It starts at
  login, hidden in the tray, by default — untick **Start at login** to stop that.

Accounts saved by earlier versions of this tool are picked up automatically.

### How it works

Claude Desktop is an Electron app, and its login lives in its profile folder.
Each account gets its own permanent folder, `~/.claude-accounts/<id>/instance/`,
and is started with Claude's `--user-data-dir=<that folder>` — so every account
stays signed in independently and several can run side by side.

The Code tab's session list (`claude-code-sessions/` inside each profile) is a
link to one shared folder, `~/.claude-instances/shared-code-sessions/`. The
list entries only point at transcripts in `~/.claude/`, which every account
already shares, so any session can be continued from any account. The first time
an account is opened by the app, its existing list is merged in (newer wins) and
the original is kept in `~/.claude-instances/session-backup/`.

Usage and the email of a new account come from `api.anthropic.com`, using the
access token Claude Desktop already stores (decrypted locally the same way
Claude does — Windows DPAPI / macOS Keychain). The token is never written,
refreshed, logged, or sent anywhere else.

### Caveats

- **Don't work in the same Code session from two accounts at the same time** —
  both would write the same session file.
- A new session started in one running account appears in another one after
  that one is restarted.
- **Stop** ends that account's Claude and any Code session running in it.
- A desktop *scheduled task* is shared too, so it runs in every account that is
  open.
- Profiles are machine-bound (encrypted with your OS user's keys). Don't copy
  `~/.claude-accounts` to another computer and never commit it — it contains
  live logins.
- Anthropic can still ask you to sign in again (session expiry, security
  checks, organization policy). This tool can't and doesn't bypass that.

### Build from source

Needs Node + pnpm, Rust (stable) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```bash
cd app
pnpm install
pnpm app          # run in development
pnpm app:build    # installer in app/src-tauri/target/release/bundle/
```

Checks:

```bash
cd app/src-tauri
cargo test                               # unit tests
cargo test probe -- --ignored --nocapture  # read-only check against this machine's accounts
```

Releases are built by GitHub Actions (`.github/workflows/release.yml`) when a
`v*` tag is pushed: a Windows installer and a universal macOS dmg, attached to
a draft release.

---

## Claude Code CLI switcher

**Install** (from this folder):

```powershell
.\install.ps1                 # Windows PowerShell
```

```bash
pwsh -c ./install.ps1         # macOS / Linux (needs PowerShell: https://aka.ms/powershell)
```

**Save each account** (one-time):

1. In Claude Code, `/login` as your first account.
2. Run `claude-code-add` → enter its email.
3. Answer **y** to "Add ANOTHER account?" — your login is cleared locally (not
   logged out), so the saved one stays valid.
4. `/login` as the next account, run `claude-code-add` again. Repeat; answer
   **n** on the last.

**Switch + continue your work:**

```powershell
claude-code-switch     # pick the account (shows usage; * = current)
claude --resume        # continue the same conversation under the new account
```

Project transcripts live in `~/.claude/projects/` (shared across accounts), so
`claude --resume` picks up exactly where you left off. Close any open Claude
Code session before switching so it re-reads the new login.
`claude-code-doctor` checks the setup.

**Updating** — auto-update is disabled for this vendored copy:
`claude-switch-update` only prints a reminder to review the upstream diff and
re-run the local `install.ps1`.

**Uninstall** — `.\uninstall.ps1` removes the CLI switcher (and leftovers of the
retired desktop switcher). Saved accounts are left alone.

---

- **Use only accounts you own.** This is for moving between your own accounts,
  not sharing accounts or evading limits.

## License

MIT — see [LICENSE](LICENSE).
