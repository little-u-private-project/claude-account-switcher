# Seatdeck

**Your work and personal Claude accounts, side by side** — for Claude Desktop & Claude Code.

> **Unofficial.** Seatdeck is an independent project. It is not affiliated with, endorsed or sponsored by Anthropic. Claude is a trademark of Anthropic, PBC.
>
> **ไม่เป็นทางการ** — ไม่ได้เกี่ยวข้อง และไม่ได้รับการรับรองจาก Anthropic

**ภาษาไทย** · [English](#english)

**[⬇ ดาวน์โหลด / Download (Releases)](https://github.com/little-u-private-project/claude-account-switcher/releases/latest)**

---

## ภาษาไทย

เปิด Claude account ที่ทำงานกับ account ส่วนตัวไว้พร้อมกัน แต่ละ account อยู่ในหน้าต่าง Claude Desktop ของตัวเอง และ login ค้างไว้ตลอด ไม่ต้อง logout แล้ว login สลับไปมาอีก

- **แอป Seatdeck** *(Windows · macOS)* — ทุก account ของ Claude Desktop login ค้างไว้ แต่ละ account เปิดเป็นหน้าต่างแยกกัน คลิก (หรือกด hotkey) เพื่อเปิด และเห็น usage 5 ชั่วโมง / 7 วันของทุก account ในที่เดียว
- **Claude Code CLI switcher** *(Windows · macOS · Linux)* — สลับ account ที่คำสั่ง `claude` ใช้งานอยู่ พอชน limit ก็สลับแล้ว `claude --resume` งานเดิมต่อได้เลย

| | Windows | macOS | Linux |
|---|:---:|:---:|:---:|
| แอป Seatdeck (Claude Desktop) | ✅ | ✅ | — |
| Claude Code CLI switcher | ✅ | ✅ | ✅ |

### แอป Seatdeck

#### ติดตั้ง

ต้องติดตั้ง Claude Desktop ไว้ก่อน (จาก Microsoft Store / claude.ai บน Windows, หรือ `/Applications/Claude.app` บน macOS) แล้วดาวน์โหลดตัวติดตั้งจากหน้า **[Releases](https://github.com/little-u-private-project/claude-account-switcher/releases/latest)**

- **Windows** — `Seatdeck_<version>_x64-setup.exe` ติดตั้งเฉพาะ user ตัวเอง ไม่ต้องใช้สิทธิ์ admin
  ตัวติดตั้งยังไม่ได้ code-sign ถ้า SmartScreen ขึ้นว่า "Windows protected your PC" ให้กด **More info → Run anyway**
- **macOS** — `Seatdeck_<version>_universal.dmg` ลากแอปไปไว้ใน Applications
  ยังไม่ได้ notarize ครั้งแรกให้คลิกขวาที่แอป → **Open** (หรือรัน `xattr -dr com.apple.quarantine /Applications/Seatdeck.app`)

ตั้งแต่ 4.3.0 แอปเช็กเวอร์ชันใหม่เองตอนเปิด (และจากเมนู **⋯ → Check for updates**) และถามก่อนติดตั้งทุกครั้ง

**อัปเกรดจาก Claude Accounts** (ชื่อเดิมของแอปนี้จนถึง 4.2.x) — account, ค่าตั้ง และข้อมูลทั้งหมดยังอยู่ครบ (`~/.claude-accounts`, `~/.claude-instances` ไม่เปลี่ยน)
- Windows: ติดตั้ง Seatdeck ทับได้เลย ตัวติดตั้งจะปิดและถอน Claude Accounts ตัวเดิมออกให้ และย้าย Start at login มาให้ด้วย
- macOS: ปิด Claude Accounts ก่อน แล้วลบ `Claude Accounts.app` ออกจาก Applications เองหลังติดตั้ง Seatdeck

#### Usage ของทุก account

แถบ usage ช่วง 5 ชั่วโมงและ 7 วันของแต่ละ account พร้อมเวลา reset อัปเดตทุก 5 นาที (สีเหลืองตั้งแต่ 80 %, สีแดงตั้งแต่ 95 %) และมีแจ้งเตือนเมื่อใกล้เต็มหรือ reset แล้ว account ที่ปิดไว้นานอาจขึ้น "expired" จนกว่าจะเปิดมันสักครั้ง

#### แชร์ session ของแท็บ Code (เลือกเปิดเอง)

ค่าเริ่มต้น แต่ละ account เห็นเฉพาะ session ของตัวเอง เปิดได้ที่ **⋯ (ด้านล่าง) → Share Code-tab sessions…** แล้วทุก account จะเห็นและ resume session ของกันและกันได้ (ถ้าเคยใช้ Claude Accounts ที่แชร์อยู่แล้ว จะยังเปิดอยู่เหมือนเดิม) ปิดเมื่อไหร่ก็ได้ — ปิดแล้วแค่หยุด sync ไม่มีอะไรถูกลบ

ทุกครั้งที่เปิด account ที่ปิดอยู่ แอปจะ backup รายการ session ของ account นั้นก่อน (เก็บ 10 ครั้งล่าสุด กู้คืนได้จาก **⋯ → Restore session backup…**) แล้วค่อย sync session ที่ลบใน account หนึ่งจะหายจากทุก account (ไม่เด้งกลับมา) และแยก org กันเสมอ

ความเสี่ยงที่ยังมี:
- **เปิด session เดียวกันจากสอง account พร้อมกัน** — ใครบันทึกทีหลังคนนั้นชนะ แอปเตือนให้ไม่ได้ เพราะมองไม่เห็นว่าหน้าต่างไหนเปิด session ไหนอยู่ (Claude ไม่ได้ใส่ session id ไว้ใน command line ของ process)
- **ไม่ live** — session ใหม่ใน account หนึ่ง จะไปโผล่ในอีก account ตอนเปิด account นั้นจากแอปครั้งถัดไป
- **Claude Desktop อัปเดตแล้วรูปแบบไฟล์อาจเปลี่ยน** — ถ้าแอปเจอไฟล์ที่ไม่รู้จัก จะหยุด sync เองและบอกเหตุผล

#### ฟีเจอร์ทั้งหมด

- **Add account** — เปิดหน้าต่าง Claude ใหม่ที่ว่างเปล่า login ในนั้นครั้งเดียว แอปจะจับได้และบันทึก account ไว้ให้เอง (ไม่ปิดหน้าต่างอื่น)
- **คลิกที่ account** เพื่อเปิด ถ้าเปิดอยู่แล้วหน้าต่างจะเด้งขึ้นมาด้านหน้า จุดสีเขียว = กำลังรันอยู่
- **เมนู ⋯ ของ account** (หรือคลิกขวา) — Stop, Rename, global hotkey, สี, Move to top, Refresh usage ลากเพื่อเรียงลำดับได้
- **เมนู ⋯ ด้านล่าง** — แชร์ session, Restore session backup, **Check & repair** (หา Claude ไม่เจอ, folder ที่เป็น link, การเพิ่ม account ที่ค้าง, email ซ้ำ, hotkey ชนกัน, session ที่ชี้ไปที่ place ของ account อื่น, ไฟล์ session ผิดรูปแบบ — ซ่อมเฉพาะที่ปลอดภัย หลัง backup), Check for updates, About
- **Open best account** — เปิด account ที่ quota เหลือมากที่สุด (ปุ่ม Open best, เมนู tray หรือ hotkey)
- **Tray** — ปิดหน้าต่างแล้วแอปยังรันอยู่ใน tray (menu bar บน macOS) เพื่อให้ hotkey ยังใช้ได้ ปุ่ม **Quit** อยู่ในเมนู tray ค่าเริ่มต้นคือเปิดอัตโนมัติตอน login แบบซ่อนใน tray — เอาติ๊ก **Start at login** ออกถ้าไม่ต้องการ

#### ข้อควรระวัง

- **Stop** จะปิด Claude ของ account นั้น รวมถึง Code session ที่รันอยู่ในนั้นด้วย
- profile ผูกกับเครื่อง (เข้ารหัสด้วย key ของ user ในระบบ) อย่า copy `~/.claude-accounts` ไปเครื่องอื่น และห้าม commit เด็ดขาด — ในนั้นมี login ที่ใช้งานได้จริง
- Anthropic ยังอาจขอให้ login ใหม่ได้ (session หมดอายุ, ตรวจสอบความปลอดภัย, นโยบายองค์กร) เครื่องมือนี้ไม่ได้และไม่พยายาม bypass สิ่งเหล่านั้น

#### ทำงานยังไง

- แต่ละ account มี profile folder ของตัวเองที่ `~/.claude-accounts/<id>/instance/` และเปิด Claude ด้วย `--user-data-dir=<folder นั้น>` เลย login แยกกันและรันพร้อมกันได้
- การแชร์ session ใช้การ copy ไฟล์ `local_*.json` ผ่าน folder กลางแยกตาม org ที่ `~/.claude-instances/shared-code-sessions/<org>/` (ไฟล์ที่ใหม่กว่าชนะ) — ไม่ใช่ link เพราะ Claude Desktop ไม่ยอมบันทึกลง folder ที่เป็น link
- usage และ email ดึงจาก `api.anthropic.com` ด้วย access token ที่ Claude Desktop เก็บไว้อยู่แล้ว (ถอดรหัสบนเครื่องแบบเดียวกับที่ Claude ทำ — Windows DPAPI / macOS Keychain) token ไม่ถูกเขียน, refresh, log หรือส่งไปที่อื่นเลย

รายละเอียดเพิ่มเติมและวิธี build เอง ดูที่ [English → How it works / Build from source](#how-it-works)

### Claude Code CLI switcher

**ติดตั้ง** (รันจาก folder นี้):

```powershell
.\install.ps1                 # Windows PowerShell
```

```bash
pwsh -c ./install.ps1         # macOS / Linux (ต้องมี PowerShell: https://aka.ms/powershell)
```

**บันทึกแต่ละ account** (ทำครั้งเดียว):

1. ใน Claude Code พิมพ์ `/login` ด้วย account แรก
2. รัน `claude-code-add` → ใส่ email ของ account นั้น
3. ตอบ **y** ที่ "Add ANOTHER account?" — login จะถูกล้างแค่ในเครื่อง (ไม่ได้ logout) account ที่บันทึกไว้จึงยังใช้ได้
4. `/login` ด้วย account ถัดไป แล้วรัน `claude-code-add` อีกครั้ง ทำซ้ำไปเรื่อย ๆ ตอบ **n** ที่ account สุดท้าย

**สลับ account แล้วทำงานต่อ:**

```powershell
claude-code-switch     # เลือก account (แสดง usage; * = อันที่ใช้อยู่)
claude --resume        # ทำ conversation เดิมต่อด้วย account ใหม่
```

ปิด Claude Code session ที่เปิดอยู่ก่อนสลับ เพื่อให้มันอ่าน login ใหม่ · `claude-code-doctor` ใช้ตรวจการตั้งค่า · `.\uninstall.ps1` ใช้ถอนการติดตั้ง (account ที่บันทึกไว้จะไม่ถูกลบ)

---

> **ใช้เฉพาะ account ที่เป็นของตัวเองเท่านั้น** เครื่องมือนี้มีไว้สลับระหว่าง account ของตัวเอง ไม่ใช่สำหรับแชร์ account หรือหลบ limit

---

## English

Keep your work and personal Claude accounts open side by side. Each account
lives in its own Claude Desktop window and stays signed in — no more logging
out and back in.

- **Seatdeck** app *(Windows · macOS)* — keeps every Claude Desktop account
  signed in, each in its own window. Click (or press a hotkey) to open one, and
  see every account's 5-hour / 7-day usage at a glance.
- **Claude Code CLI switcher** *(Windows · macOS · Linux)* — swaps which
  account `claude` runs under, so you can hit a limit, switch, and
  `claude --resume` the same work.

| | Windows | macOS | Linux |
|---|:---:|:---:|:---:|
| Seatdeck app (Claude Desktop) | ✅ | ✅ | — |
| Claude Code CLI switcher | ✅ | ✅ | ✅ |

### Seatdeck (Claude Desktop)

#### Install

Claude Desktop itself must be installed (Microsoft Store / claude.ai download on
Windows, `/Applications/Claude.app` on macOS). Then download the installer from
the **[Releases](https://github.com/little-u-private-project/claude-account-switcher/releases/latest)** page:

- **Windows** — `Seatdeck_<version>_x64-setup.exe` (installs for your user
  only, no admin). The build is not code-signed: if SmartScreen says "Windows
  protected your PC", click **More info → Run anyway**.
- **macOS** — `Seatdeck_<version>_universal.dmg`, drag the app to
  Applications. Not notarized: the first time, right-click the app → **Open**
  (or run `xattr -dr com.apple.quarantine /Applications/Seatdeck.app`).

From 4.3.0 the app looks for a new version when it starts (and from
**⋯ → Check for updates**), and always asks before installing it.

**Upgrading from Claude Accounts** (this app's name up to 4.2.x): your
accounts, settings and data stay (`~/.claude-accounts` and
`~/.claude-instances` are unchanged).
- Windows: just install Seatdeck. The installer closes and removes the old
  Claude Accounts install and carries Start at login over.
- macOS: quit Claude Accounts first, then delete `Claude Accounts.app` from
  Applications after installing Seatdeck.

#### Usage for every account

5-hour and 7-day plan usage bars per account, with reset times, refreshed every
5 minutes (amber from 80 %, red from 95 %), plus a notification when a window
nears its limit or resets. An account that has been closed for a long time may
show "expired" until you open it once.

#### Sharing Code-tab sessions (opt-in)

By default each account sees only its own Code-tab sessions. Turn sharing on
under **⋯ (bottom) → Share Code-tab sessions…** and every account lists — and
can resume — every account's sessions. (Installs that already shared sessions
under Claude Accounts keep sharing.) Turning it off only stops syncing; nothing
is deleted.

Each time a stopped account is opened, its session list is backed up first
(the last 10 are kept; **⋯ → Restore session backup…** puts one back), then
synced. A session deleted in one account is deleted everywhere and doesn't come
back, and different orgs never mix.

Risks that remain:
- **The same session open in two accounts at once** — the last one to save
  wins. Seatdeck can't warn you: it can't see which session a window has open
  (Claude doesn't put the session id on the process command line).
- **Not live** — a new session in one account shows up in another the next time
  that one is opened from Seatdeck.
- **A Claude Desktop update may change the file format** — if Seatdeck finds a
  session file it doesn't recognise, sync turns itself off and says why.

#### All features

- **Add account** — opens a fresh, empty Claude window. Sign in there once; the
  app notices and saves the account. Nothing else is closed.
- **Click an account** to open it. If it is already open, its window comes to
  the front. The dot is green while it runs.
- **An account's ⋯ menu** (or right-click) — Stop, Rename, global hotkey,
  colour, Move to top, Refresh usage. Drag rows to reorder.
- **The ⋯ menu at the bottom** — session sharing, Restore session backup,
  **Check & repair** (Claude not found, linked session folders, unfinished
  adds, duplicate emails, hotkey conflicts, sessions pointing at another
  account's saved place, session files of an unknown format; repairs only what
  is safe, after a backup), Check for updates, About.
- **Open best account** — opens the account with the most quota left (Open best
  button, tray menu or a hotkey).
- **Tray** — closing the window keeps the app running in the tray (menu bar on
  macOS) so hotkeys keep working; **Quit** is in the tray menu. It starts at
  login, hidden in the tray, by default — untick **Start at login** to stop that.

Accounts saved by earlier versions of this tool are picked up automatically.

#### How it works

Claude Desktop is an Electron app, and its login lives in its profile folder.
Each account gets its own permanent folder, `~/.claude-accounts/<id>/instance/`,
and is started with Claude's `--user-data-dir=<that folder>` — so every account
stays signed in independently and several can run side by side.

Session sharing copies the Code tab's list entries (`local_*.json` in
`claude-code-sessions/<account>/<org>/` inside each profile) through a hub per
org, `~/.claude-instances/shared-code-sessions/<org>/` (newer file wins). The
entries only point at transcripts in `~/.claude/`, which every account already
shares, so any session can be continued from any account. It copies rather
than links because Claude Desktop refuses to save into a linked folder.
Deletes are tracked per account (`sync-state.json` in the hub), and backups go
to `~/.claude-instances/session-backup/`.

Usage and the email of a new account come from `api.anthropic.com`, using the
access token Claude Desktop already stores (decrypted locally the same way
Claude does — Windows DPAPI / macOS Keychain). The token is never written,
refreshed, logged, or sent anywhere else.

#### Caveats

- **Stop** ends that account's Claude and any Code session running in it.
- Profiles are machine-bound (encrypted with your OS user's keys). Don't copy
  `~/.claude-accounts` to another computer and never commit it — it contains
  live logins.
- Anthropic can still ask you to sign in again (session expiry, security
  checks, organization policy). This tool can't and doesn't bypass that.

#### Build from source

Needs Node + pnpm, Rust (stable) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```bash
cd app
pnpm install
pnpm app          # run in development
pnpm app:build    # installer in app/src-tauri/target/release/bundle/
```

`pnpm app:build` also signs the update bundle, so it needs the updater key:
set `TAURI_SIGNING_PRIVATE_KEY` (the key file's contents or path) and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, or turn `createUpdaterArtifacts` off in
`tauri.conf.json` for a local build.

Checks:

```bash
cd app/src-tauri
cargo test                               # unit tests
cargo test probe -- --ignored --nocapture  # read-only check against this machine's accounts
```

Releases are built by GitHub Actions (`.github/workflows/release.yml`) when a
`v*` tag is pushed: a Windows installer and a universal macOS dmg with signed
update bundles and `latest.json`, attached to a draft release. Publishing the
release is what makes installed copies offer the update.

### Claude Code CLI switcher

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

> **Use only accounts you own.** This is for moving between your own accounts,
> not sharing accounts or evading limits.

## License

MIT — see [LICENSE](LICENSE).
