import { invoke } from "@tauri-apps/api/core";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { Effect, EffectState, getCurrentWindow } from "@tauri-apps/api/window";

type Account = { id: string; email: string; displayName: string | null; hotkey: string | null; running: boolean };
type Window = { utilization: number; resets_at: string | null } | null;
type Usage = { five_hour?: Window; seven_day?: Window };

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const list = $<HTMLElement>("list");
const isMac = navigator.userAgent.includes("Mac");
// macOS styles the window as a menu-bar popover on Liquid Glass (style.css, html.mac).
document.documentElement.classList.toggle("mac", isMac);

let accounts: Account[] = [];
const usage = new Map<string, Usage | string>();

let toastTimer: ReturnType<typeof setTimeout> | undefined;
function toast(msg: string) {
  const t = $<HTMLElement>("toast");
  t.textContent = msg;
  t.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (t.hidden = true), 5000);
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    toast(String(e));
  }
}


function prettyHotkey(h: string) {
  return h
    .split("+")
    .map((p) => p.replace(/^Key/, "").replace(/^Digit/, "").replace(/^Super$/, isMac ? "Cmd" : "Win"))
    .join("+");
}

function resetIn(iso: string | null | undefined, weekly: boolean) {
  if (!iso) return "";
  const at = new Date(iso);
  const mins = Math.round((at.getTime() - Date.now()) / 60000);
  if (mins <= 0) return "";
  if (weekly) return at.toLocaleString(undefined, { weekday: "short", hour: "2-digit", minute: "2-digit" });
  return mins >= 60 ? `${Math.floor(mins / 60)}h${String(mins % 60).padStart(2, "0")}m` : `${mins}m`;
}

function bar(label: string, w: Window | undefined, weekly: boolean) {
  if (!w) return "";
  const pct = Math.round(w.utilization);
  const level = pct >= 95 ? "full" : pct >= 80 ? "high" : "";
  const reset = resetIn(w.resets_at, weekly);
  return `<div class="bar ${level}" title="${label} window${reset ? ` · resets ${reset}` : ""}">
    <span class="k">${label}</span>
    <span class="track"><span class="fill" style="width:${Math.min(pct, 100)}%"></span></span>
    <span class="v">${pct}%</span>
    <span class="r">${reset ? `↻ ${reset}` : ""}</span>
  </div>`;
}

function esc(s: string) {
  return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);
}


function render() {
  if (!accounts.length) {
    list.innerHTML = `<div class="empty">No saved accounts yet.<br />Click <b>Add account</b> to sign one in.</div>`;
    return;
  }
  list.innerHTML = accounts
    .map((a) => {
      const name = a.displayName || a.email;
      const u = usage.get(a.id);
      const usageHtml =
        typeof u === "string"
          ? `<div class="usage-err">${esc(u)}</div>`
          : u
            ? bar("5h", u.five_hour, false) + bar("7d", u.seven_day, true)
            : `<div class="usage-err">loading usage…</div>`;
      return `<div class="row ${a.running ? "on" : ""}" data-id="${esc(a.id)}" tabindex="0">
        <span class="dot" title="${a.running ? "Running" : "Not running"}"></span>
        <div class="body">
          <div class="name">${esc(name)}${a.hotkey ? `<span class="chip">${esc(prettyHotkey(a.hotkey))}</span>` : ""}</div>
          <div class="sub">${a.displayName ? esc(a.email) + " · " : ""}${a.running ? "Running" : "Not running"}</div>
          <div class="usage">${usageHtml}</div>
        </div>
        <button class="more" data-menu="${esc(a.id)}" title="More">⋯</button>
      </div>`;
    })
    .join("");
}

async function refresh() {
  const next = await call<Account[]>("list_accounts");
  if (!next) return;
  const changed = JSON.stringify(next) !== JSON.stringify(accounts);
  accounts = next;
  if (changed) render();
}

async function fetchUsage(id: string) {
  try {
    usage.set(id, await invoke<Usage>("account_usage", { id }));
  } catch (e) {
    usage.set(id, String(e));
  }
}

async function loadUsage(id: string) {
  await fetchUsage(id);
  render();
}

let usageAt = 0;
async function loadAllUsage() {
  usageAt = Date.now();
  await Promise.all(accounts.map((a) => fetchUsage(a.id)));
  render();
}


const dialog = $<HTMLDialogElement>("dialog");
// Enter means OK (the form's first button is Cancel, which Enter would pick).
$<HTMLInputElement>("d-input").addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.ctrlKey && !e.altKey && !e.metaKey) {
    e.preventDefault();
    dialog.close("ok");
  }
});

type DialogOpts = { title: string; text: string; input?: string | null; ok?: string; cancel?: string | null };

function openDialog(o: DialogOpts): Promise<string | null> {
  $("d-title").textContent = o.title;
  $("d-text").textContent = o.text;
  const input = $<HTMLInputElement>("d-input");
  input.hidden = o.input === undefined;
  input.value = o.input ?? "";
  input.readOnly = false;
  $("d-ok").textContent = o.ok ?? "OK";
  $("d-ok").hidden = false;
  $("d-cancel").textContent = o.cancel ?? "Cancel";
  $("d-cancel").hidden = o.cancel === null;
  dialog.returnValue = "";
  dialog.showModal();
  if (!input.hidden) input.select();
  return new Promise((resolve) =>
    dialog.addEventListener("close", () => resolve(dialog.returnValue === "ok" ? input.value : null), { once: true }),
  );
}

async function rename(a: Account) {
  const name = await openDialog({ title: "Rename", text: a.email, input: a.displayName ?? "", ok: "Save" });
  if (name === null) return;
  await call("rename_account", { id: a.id, name: name || null });
  refresh();
}

async function setHotkey(a: Account) {
  const input = $<HTMLInputElement>("d-input");
  let combo = a.hotkey ?? "";
  const onKey = (e: KeyboardEvent) => {
    if (["Tab", "Enter", "Escape"].includes(e.key) && !e.ctrlKey && !e.altKey && !e.metaKey) return;
    e.preventDefault();
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    // Shift alone is not enough: a global Shift+A would swallow every capital A typed anywhere.
    if (/^(Control|Alt|Shift|Meta|OS)/.test(e.code) || !mods.some((m) => m !== "Shift")) {
      input.value = mods.length ? mods.join("+") + "+…" : "Hold Ctrl / Alt / " + (isMac ? "Cmd" : "Win") + " and press a key";
      return;
    }
    combo = [...mods, e.code].join("+");
    input.value = prettyHotkey(combo);
  };
  input.addEventListener("keydown", onKey);
  const p = openDialog({
    title: "Hotkey",
    text: `Press the shortcut that opens ${a.displayName || a.email} from anywhere.`,
    input: combo ? prettyHotkey(combo) : "",
    ok: "Save",
  });
  input.readOnly = true;
  input.focus();
  const result = await p;
  input.removeEventListener("keydown", onKey);
  if (result === null || !combo) return;
  await call("set_hotkey", { id: a.id, hotkey: combo });
  refresh();
}

async function addAccount(resumeId?: string) {
  const id = resumeId ?? (await call<string>("add_start"));
  if (!id) return;
  let done = false;
  let stop = false;
  const p = openDialog({
    title: "Add account",
    text: resumeId
      ? "You started adding an account last time and its Claude window is still open. Sign in there — it is saved here automatically."
      : "A new, empty Claude window is opening. Sign in there to the account you want to add — it is saved here automatically. Nothing else is closed.",
    cancel: "Cancel",
    ok: "Done",
  });
  $("d-ok").hidden = true;
  (async () => {
    while (!stop) {
      await new Promise((r) => setTimeout(r, 2000));
      if (stop) return;
      try {
        const email = await invoke<string | null>("add_poll", { id });
        if (email) {
          done = true;
          $("d-text").textContent = `Saved ${email}. Its window stays open and signed in.`;
          $("d-cancel").hidden = true;
          $("d-ok").hidden = false;
          refresh().then(() => loadUsage(id));
          return;
        }
      } catch (e) {
        // Keep polling: a network blip must not leave Cancel (which deletes) as the only way out.
        $("d-text").textContent = String(e);
      }
    }
  })();
  await p;
  stop = true;
  if (!done) await call("add_cancel", { id });
}


let menu: HTMLElement | null = null;
const closeMenu = () => {
  menu?.remove();
  menu = null;
};

function showMenu(a: Account, anchor: HTMLElement) {
  closeMenu();
  const items: [string, () => void][] = [
    [a.running ? "Bring to front" : "Open", () => openAccount(a)],
    ...(a.running ? [["Stop", () => stopAccount(a)] as [string, () => void]] : []),
    ["Rename…", () => rename(a)],
    [a.hotkey ? "Change hotkey…" : "Set hotkey…", () => setHotkey(a)],
    ...(a.hotkey
      ? [["Remove hotkey", () => call("set_hotkey", { id: a.id, hotkey: null }).then(refresh)] as [string, () => void]]
      : []),
    ["Refresh usage", () => loadUsage(a.id)],
  ];
  menu = document.createElement("div");
  menu.className = "menu";
  for (const [label, fn] of items) {
    const b = document.createElement("button");
    b.textContent = label;
    b.onclick = () => {
      closeMenu();
      fn();
    };
    menu.append(b);
  }
  document.body.append(menu);
  const r = anchor.getBoundingClientRect();
  const top = Math.min(r.bottom + 4, innerHeight - menu.offsetHeight - 8);
  menu.style.top = `${Math.max(8, top)}px`;
  menu.style.left = `${Math.max(8, r.right - menu.offsetWidth)}px`;
}

async function openAccount(a: Account) {
  await call("open_account", { id: a.id });
  setTimeout(refresh, 1500);
}

async function stopAccount(a: Account) {
  const ok = await openDialog({
    title: "Stop " + (a.displayName || a.email) + "?",
    text: "Closes this account's Claude window and any Code session running in it.",
    ok: "Stop",
  });
  if (ok === null) return;
  await call("stop_account", { id: a.id });
  setTimeout(refresh, 800);
}


const rowOf = (e: Event) => (e.target as HTMLElement).closest<HTMLElement>(".row");
const accountOf = (e: Event) => accounts.find((x) => x.id === rowOf(e)?.dataset.id);

list.addEventListener("click", (e) => {
  const a = accountOf(e);
  if (!a) return;
  const more = (e.target as HTMLElement).closest<HTMLElement>("[data-menu]");
  if (more) showMenu(a, more);
  else openAccount(a);
});
list.addEventListener("contextmenu", (e) => {
  const a = accountOf(e);
  if (!a) return;
  e.preventDefault();
  showMenu(a, rowOf(e)!.querySelector(".more")!);
});
list.addEventListener("keydown", (e) => {
  const a = accountOf(e);
  if (a && e.key === "Enter") openAccount(a);
});
document.addEventListener("click", (e) => {
  if (menu && !menu.contains(e.target as Node) && !(e.target as HTMLElement).closest("[data-menu]")) closeMenu();
});
document.addEventListener("keydown", (e) => e.key === "Escape" && closeMenu());
$("add").addEventListener("click", () => addAccount());
$("quit").addEventListener("click", () => call("quit"));

const autostart = $<HTMLInputElement>("autostart");
invoke<boolean>("autostart_enabled").then((on) => (autostart.checked = on));
autostart.addEventListener("change", async () => {
  if ((await call("set_autostart", { on: autostart.checked })) === undefined) {
    autostart.checked = await invoke<boolean>("autostart_enabled");
  }
});

// The app was closed in the middle of adding an account: keep going if that
// window is still open, otherwise save it if it got signed in, else clean up.
async function resumePending() {
  for (const [id, running] of (await call<[string, boolean][]>("pending_accounts")) ?? []) {
    if (running) await addAccount(id);
    // An error (offline at login, duplicate) leaves it for the next start - only a sure "not signed in" deletes.
    else if ((await invoke<string | null>("add_poll", { id }).catch(() => undefined)) === null) await call("add_cancel", { id });
  }
}

await refresh();
render();
loadAllUsage();
resumePending().then(refresh);
// The window mostly sits hidden in the tray - only poll while it is shown.
// (WebView2 keeps document.hidden false when the window is hidden, so ask Tauri.)
const win = getCurrentWindow();
const shown = () => win.isVisible().catch(() => true);
setInterval(async () => (await shown()) && refresh(), 5000);
setInterval(async () => (await shown()) && loadAllUsage(), 5 * 60 * 1000);
win.onFocusChanged(({ payload: focused }) => {
  if (!focused) return;
  refresh();
  if (Date.now() - usageAt > 60 * 1000) loadAllUsage();
});

// macOS: the popover is only as tall as its content (HIG: popovers), with room
// for the ⋯ menu and dialogs; past the cap the page scrolls.
if (isMac) {
  const fit = () => win.setSize(new LogicalSize(innerWidth, Math.min(Math.max(document.body.scrollHeight, 280), 640)));
  new ResizeObserver(fit).observe(document.body);
}

// macOS background: frosted (Liquid Glass regular, the tauri.macos.conf.json
// default) or clear (Liquid Glass clear, the see-through iOS 27 look). Kept per
// machine in the webview's storage.
if (isMac) {
  const glass = $<HTMLElement>("glass");
  const apply = (clear: boolean, save: boolean) => {
    document.documentElement.classList.toggle("clear", clear);
    for (const b of glass.querySelectorAll("button")) b.ariaPressed = String((b.dataset.v === "clear") === clear);
    if (!save) return;
    localStorage.setItem("glass", clear ? "clear" : "frosted");
    win.setEffects({ effects: [clear ? Effect.LiquidGlassClear : Effect.LiquidGlassRegular, Effect.Popover], state: EffectState.Active, radius: 16 });
  };
  glass.hidden = false;
  glass.addEventListener("click", (e) => {
    const v = (e.target as HTMLElement).closest("button")?.dataset.v;
    if (v) apply(v === "clear", true);
  });
  const clear = localStorage.getItem("glass") === "clear";
  apply(clear, clear);
}

// Theme: follow the system (default) or force light / dark. setTheme switches
// the whole app's appearance - the glass and the webview's prefers-color-scheme
// with it - so the CSS needs no theme switch of its own.
const themes = ["auto", "light", "dark"] as const;
type Theme = (typeof themes)[number];
const themeBtn = $<HTMLButtonElement>("theme");
const applyTheme = (t: Theme) => {
  themeBtn.dataset.theme = t;
  themeBtn.title = `Theme: ${t === "auto" ? "match system" : t}`;
  win.setTheme(t === "auto" ? null : t);
};
themeBtn.addEventListener("click", () => {
  const next = themes[(themes.indexOf(themeBtn.dataset.theme as Theme) + 1) % themes.length];
  localStorage.setItem("theme", next);
  applyTheme(next);
});
const savedTheme = localStorage.getItem("theme") as Theme;
if (themes.includes(savedTheme) && savedTheme !== "auto") applyTheme(savedTheme);
else themeBtn.title = "Theme: match system";
