# seatdeck-cli — behaviour spec

What the Claude Code account switcher must do. The implementation in
`src/main.rs` follows this document.

## Where Claude Code keeps its login

- **Credentials** (the OAuth tokens):
  - Windows, Linux: the JSON file `<config>/.credentials.json`, where
    `<config>` is `$CLAUDE_CONFIG_DIR` or else `~/.claude`.
  - macOS: a generic password in the login Keychain, service
    `Claude Code-credentials`, account `$USER`. The password is the same
    JSON text.
  - The JSON holds `claudeAiOauth` (`accessToken`, `refreshToken`,
    `expiresAt` in ms, `scopes`, `subscriptionType`, `rateLimitTier`), an
    optional `organizationUuid`, and other keys that belong to the machine,
    not the account (for example `mcpOAuth`, logins to MCP servers).
- **Identity** shown by `/status`: the `oauthAccount` object in the global
  config `$CLAUDE_CONFIG_DIR/.claude.json` or else `~/.claude.json`.
  `oauthAccount.emailAddress` is the email of the current login.
- Claude Code rotates the refresh token every time it refreshes the access
  token, so a saved copy of an account goes stale once that account is used.

## Saved accounts

- Folder `~/.claude-cc-accounts/` (the name the earlier PowerShell switcher
  used, so accounts saved by it keep working as they are).
- One file per account, `<email>.json`, with characters other than letters,
  digits and `_ . @ + -` turned into `_`. Fields:
  `email`, `savedAt` (RFC 3339), `credentials` (the whole credentials object
  at save time) and, when known, `oauthAccountRaw` (the `oauthAccount` object
  as JSON text).
- Files may start with a UTF-8 BOM (written by Windows PowerShell 5.1); it is
  ignored. Files without `email` and `credentials`, and names starting with
  `backup-`, are not accounts.
- `.backups/` keeps the last 5 copies of the live credentials taken before a
  switch, and `claude.json.bak`, the global config as it was before the last
  edit. `.current` holds the email the switcher last activated.
- On macOS/Linux the folder is `0700` and the files `0600`.

## The current account

The saved account whose access token equals the live one. Failing that (the
token was refreshed since it was saved), the saved account whose email equals
`oauthAccount.emailAddress` — but only if `.current` is missing or names the
same email, so a stale identity can never pull another account's tokens into
the wrong file.

## Keeping saved copies fresh

Every `list` and `switch` first writes the live credentials (and the live
`oauthAccount`, when its email matches) back into the current account's file
if the access token changed. Without this, switching back later would restore
a revoked refresh token and log that account out.

## Commands

- `add [email] [--force]` — save the live login. The email defaults to the
  one in `oauthAccount`; none known and none given → ask. If the given email
  differs from the detected one, refuse unless `--force` (otherwise one
  account's tokens get saved under another's name). On a terminal, then offer
  to clear the local login (drop `claudeAiOauth`, keep everything else) so the
  next account can `/login`; clearing is local only, nothing is revoked.
- `list` — every saved account, `*` on the current one, its 5-hour and 7-day
  plan usage in percent and when each window resets.
- `switch [n|email]` — with no argument, show the list and ask for a number.
  Switching to the current account only re-syncs the identity. Otherwise:
  back up the live credentials, put the chosen account's `claudeAiOauth` (and
  `organizationUuid`) into the live credentials while leaving other keys such
  as `mcpOAuth` untouched, set `oauthAccount` in the global config when the
  account has one saved, and record `.current`. Then tell the user to restart
  any open Claude Code session and `claude --resume`.
- `doctor` — platform, where credentials and config are read from, whether a
  live login exists and which email it is, the saved accounts and which one is
  current, and anything that blocks switching (macOS without `security`,
  `CLAUDE_CONFIG_DIR` on macOS, a left-over PowerShell install). Never prints
  a token or part of one.

## Usage

`GET https://api.anthropic.com/api/oauth/usage` with `Authorization: Bearer
<accessToken>` and `anthropic-beta: oauth-2025-04-20`; the answer has
`five_hour` and `seven_day`, each with `utilization` (percent) and `resets_at`
(RFC 3339). Accounts are queried in parallel with a short timeout. A token
past `expiresAt` is not sent; the list says to switch to that account (Claude
Code refreshes it on its next run). The switcher never refreshes tokens
itself and never sends them anywhere else.

## Not done, on purpose

- No automatic switching when a limit is hit. Picking an account is always the
  user's action.
- No self-update.
