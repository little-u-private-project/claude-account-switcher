# Claude Account Switcher

**ภาษาไทย** · [English](#english)

**[⬇ ดาวน์โหลด / Download (Releases)](https://github.com/little-u-private-project/claude-account-switcher/releases/latest)**

---

## ภาษาไทย

ใช้ **Anthropic Claude** หลาย account ของตัวเอง (เช่น Pro/Max หลาย seat) ได้โดยไม่ต้อง logout แล้ว login ใหม่ทุกครั้ง

- **แอป Claude Accounts** *(Windows · macOS)* — ให้ทุก account ของ Claude Desktop login ค้างไว้ตลอด แต่ละ account เปิดเป็นหน้าต่างแยกกัน คลิก (หรือกด hotkey) เพื่อเปิด, ดู usage 5 ชั่วโมง / 7 วันของแต่ละ account ได้ในที่เดียว และ session ของ Claude Code จากทุก account จะเห็นได้จากทุก account
- **Claude Code CLI switcher** *(Windows · macOS · Linux)* — สลับ account ที่คำสั่ง `claude` ใช้งานอยู่ พอชน limit ก็สลับแล้ว `claude --resume` งานเดิมต่อได้เลย

| | Windows | macOS | Linux |
|---|:---:|:---:|:---:|
| แอป Claude Accounts (Claude Desktop) | ✅ | ⚠️ build แล้ว ยังไม่ได้ทดสอบจริง | — |
| Claude Code CLI switcher | ✅ | ✅ | ✅ |

### แอป Claude Accounts

#### ติดตั้ง

ต้องติดตั้ง Claude Desktop ไว้ก่อน (จาก Microsoft Store / claude.ai บน Windows, หรือ `/Applications/Claude.app` บน macOS) แล้วดาวน์โหลดตัวติดตั้งจากหน้า **[Releases](https://github.com/little-u-private-project/claude-account-switcher/releases/latest)**

- **Windows** — `Claude.Accounts_<version>_x64-setup.exe` ติดตั้งเฉพาะ user ตัวเอง ไม่ต้องใช้สิทธิ์ admin
  ตัวติดตั้งยังไม่ได้ code-sign ถ้า SmartScreen ขึ้นว่า "Windows protected your PC" ให้กด **More info → Run anyway**
- **macOS** — `Claude.Accounts_<version>_universal.dmg` ลากแอปไปไว้ใน Applications
  ยังไม่ได้ notarize ครั้งแรกให้คลิกขวาที่แอป → **Open** (หรือรัน `xattr -dr com.apple.quarantine "/Applications/Claude Accounts.app"`)

#### วิธีใช้

- **Add account** — เปิดหน้าต่าง Claude ใหม่ที่ว่างเปล่า login ในนั้นครั้งเดียว แอปจะจับได้และบันทึก account ไว้ให้เอง (ไม่ปิดหน้าต่างอื่น) ทำซ้ำทีละ account
- **คลิกที่ account** เพื่อเปิด ถ้าเปิดอยู่แล้วหน้าต่างจะเด้งขึ้นมาด้านหน้า จุดสีเขียว = กำลังรันอยู่
- **เมนู ⋯** (หรือคลิกขวา) — Stop, Rename, ตั้ง / ลบ global hotkey, Refresh usage
- **แถบ usage** — usage ของ plan ช่วง 5 ชั่วโมงและ 7 วัน พร้อมเวลา reset อัปเดตทุก 5 นาที (สีเหลืองตั้งแต่ 80 %, สีแดงตั้งแต่ 95 %) account ที่ปิดไว้นานอาจขึ้น "expired" จนกว่าจะเปิดมันสักครั้ง
- **Tray** — ปิดหน้าต่างแล้วแอปยังรันอยู่ใน tray (menu bar บน macOS) เพื่อให้ hotkey ยังใช้ได้ ปุ่ม **Quit** อยู่ในเมนู tray ค่าเริ่มต้นคือเปิดอัตโนมัติตอน login แบบซ่อนใน tray — เอาติ๊ก **Start at login** ออกถ้าไม่ต้องการ

#### ข้อควรระวัง

- **อย่าทำงานใน Code session เดียวกันจากสอง account พร้อมกัน** — ทั้งคู่จะเขียนไฟล์ session เดียวกัน
- session ใหม่ที่เริ่มใน account หนึ่ง จะไปโผล่ในอีก account ก็ต่อเมื่อ restart account นั้น
- **Stop** จะปิด Claude ของ account นั้น รวมถึง Code session ที่รันอยู่ในนั้นด้วย
- *scheduled task* ของ Claude Desktop ถูกแชร์ด้วย มันจะรันในทุก account ที่เปิดอยู่
- profile ผูกกับเครื่อง (เข้ารหัสด้วย key ของ user ในระบบ) อย่า copy `~/.claude-accounts` ไปเครื่องอื่น และห้าม commit เด็ดขาด — ในนั้นมี login ที่ใช้งานได้จริง
- Anthropic ยังอาจขอให้ login ใหม่ได้ (session หมดอายุ, ตรวจสอบความปลอดภัย, นโยบายองค์กร) เครื่องมือนี้ไม่ได้และไม่พยายาม bypass สิ่งเหล่านั้น

#### ทำงานยังไง

- แต่ละ account มี profile folder ของตัวเองที่ `~/.claude-accounts/<id>/instance/` และเปิด Claude ด้วย `--user-data-dir=<folder นั้น>` เลย login แยกกันและรันพร้อมกันได้
- ทุกครั้งที่เปิด account จากแอป รายการ session ของแท็บ Code จาก account อื่นทุกตัวจะถูก copy เข้ามา (ไฟล์ที่ใหม่กว่าชนะ ผ่าน folder กลาง `~/.claude-instances/shared-code-sessions/`) — ใช้ copy ไม่ใช่ link เพราะ Claude Desktop ไม่ยอมบันทึกลง folder ที่เป็น link
- session ที่สร้างใน account ที่เปิดอยู่จะไปโผล่ใน account อื่นตอนเปิด account นั้นครั้งถัดไป และ session ที่ลบใน account หนึ่งอาจกลับมาถ้า account อื่นยังมีอยู่
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

### Claude Accounts (Claude Desktop)

#### Install

Claude Desktop itself must be installed (Microsoft Store / claude.ai download on
Windows, `/Applications/Claude.app` on macOS). Then download the installer from
the **[Releases](https://github.com/little-u-private-project/claude-account-switcher/releases/latest)** page:

- **Windows** — `Claude.Accounts_<version>_x64-setup.exe` (installs for your
  user only, no admin). The build is not code-signed: if SmartScreen says
  "Windows protected your PC", click **More info → Run anyway**.
- **macOS** — `Claude.Accounts_<version>_universal.dmg`, drag the app to
  Applications. Not notarized: the first time, right-click the app → **Open**
  (or run `xattr -dr com.apple.quarantine "/Applications/Claude Accounts.app"`).

#### Use

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

#### How it works

Claude Desktop is an Electron app, and its login lives in its profile folder.
Each account gets its own permanent folder, `~/.claude-accounts/<id>/instance/`,
and is started with Claude's `--user-data-dir=<that folder>` — so every account
stays signed in independently and several can run side by side.

The Code tab's session list (`claude-code-sessions/` inside each profile) is
synced by copying: whenever the app opens an account, every account's list is
merged into `~/.claude-instances/shared-code-sessions/` (newer file wins) and
that folder is merged back into the account being opened. The list entries
only point at transcripts in `~/.claude/`, which every account already shares,
so any session can be continued from any account. It copies rather than links
because Claude Desktop refuses to save into a linked folder. A session created
in one account shows up in another the next time that one is opened; a session
deleted in one account can come back while another account still lists it.

Usage and the email of a new account come from `api.anthropic.com`, using the
access token Claude Desktop already stores (decrypted locally the same way
Claude does — Windows DPAPI / macOS Keychain). The token is never written,
refreshed, logged, or sent anywhere else.

#### Caveats

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

#### Build from source

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
