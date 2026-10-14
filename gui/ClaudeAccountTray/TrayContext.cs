using System;
using System.ComponentModel;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace ClaudeAccountTray
{
    // Was a hidden tray-icon app; now runs as a normal, always-visible
    // program window instead (per explicit user request - not everyone
    // wants a background tray process). Still owns the global hotkeys and
    // the PowerShell launch/stop logic; the main window is just the UI.
    public class TrayContext : ApplicationContext
    {
        private readonly MessageWindow _messageWindow;
        private readonly HotkeyManager _hotkeys;
        private readonly SwitcherPalette _mainWindow;

        private static readonly string ToolsDir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".claude-tools");
        private static readonly string StartScript = Path.Combine(ToolsDir, "claude-instance-start.ps1");
        private static readonly string StopScript = Path.Combine(ToolsDir, "claude-instance-stop.ps1");

        public TrayContext()
        {
            _messageWindow = new MessageWindow();
            // _hotkeys doesn't exist yet at this point - creating the handle
            // below pumps window-creation messages (WM_CREATE etc, never
            // WM_HOTKEY) through WndProc immediately, so this must tolerate
            // firing before _hotkeys is assigned a few lines down.
            _messageWindow.HotkeyMessage += m => { if (_hotkeys != null) _hotkeys.OnWndProc(m); };
            _messageWindow.EnsureHandleCreated();

            _hotkeys = new HotkeyManager(_messageWindow.Handle);
            _hotkeys.HotkeyPressed += email => LaunchOrFocus(email);

            _mainWindow = new SwitcherPalette(this);
            _mainWindow.Icon = LoadClaudeIcon();
            _mainWindow.FormClosed += (s, e) => ExitThread();
            _mainWindow.RefreshList();
            _mainWindow.Show();

            RegisterHotkeys();
        }

        // install.ps1 already resolves Claude.exe's path once at install time
        // (the same Get-AppxPackage lookup it uses for the Desktop shortcut
        // icon) and writes it to claude-exe-path.txt - reading that file is
        // instant, versus shelling out to PowerShell + querying AppX
        // packages again on every single app startup, which can add a
        // noticeable startup delay for no benefit (the install location
        // doesn't change between launches).
        private static Icon LoadClaudeIcon()
        {
            try
            {
                var pathFile = Path.Combine(ToolsDir, "claude-exe-path.txt");
                if (File.Exists(pathFile))
                {
                    var exe = File.ReadAllText(pathFile).Trim();
                    if (File.Exists(exe))
                    {
                        // install.ps1 now points this at a stable .ico copy (the
                        // MSIX exe path is version-stamped and dies on update).
                        if (exe.EndsWith(".ico", StringComparison.OrdinalIgnoreCase))
                            return new Icon(exe);
                        return Icon.ExtractAssociatedIcon(exe);
                    }
                }
            }
            catch { }
            return SystemIcons.Application;
        }

        private void RegisterHotkeys()
        {
            _hotkeys.RegisterAll(InstanceConfig.Load());
        }

        // Both run the underlying PowerShell script on a background thread -
        // claude-instance-start.ps1 can take several seconds on first launch
        // (it robocopies the profile to seed the isolated instance folder),
        // and running that synchronously on the UI thread would freeze the
        // window for the duration, for every launch/stop/hotkey press.
        public void LaunchOrFocus(string email)
        {
            var worker = new BackgroundWorker();
            worker.DoWork += (s, e) => RunScript(StartScript, email);
            worker.RunWorkerCompleted += (s, e) =>
            {
                RegisterHotkeys();
                _mainWindow.RefreshList();
                TryFocusWindow(email);
            };
            worker.RunWorkerAsync();
        }

        public void Stop(string email)
        {
            var worker = new BackgroundWorker();
            worker.DoWork += (s, e) => RunScript(StopScript, email);
            worker.RunWorkerCompleted += (s, e) =>
            {
                RegisterHotkeys();
                _mainWindow.RefreshList();
            };
            worker.RunWorkerAsync();
        }

        // Lets other windows (the main window's right-click row actions,
        // Settings) tell the tray to re-read instances.json and re-register
        // hotkeys after they change something themselves, without
        // duplicating that logic.
        public void RefreshAfterExternalChange()
        {
            RegisterHotkeys();
            _mainWindow.ApplyThemeAndRedraw();
        }

        private static void RunScript(string script, string email)
        {
            PsRunner.RunFireAndForget(script, "-Email", email);
        }

        // Best-effort: bring the account's main window to the foreground if we
        // can find one owned by its recorded process. Electron may run child
        // renderer processes under different PIDs than the one we launched, so
        // this can silently find nothing - the window is still open either way,
        // just not auto-focused (see blueprint Open Question #2).
        private static void TryFocusWindow(string email)
        {
            var entry = InstanceConfig.Find(InstanceConfig.Load(), email);
            if (entry == null) return;
            int? livePid;
            if (!InstanceConfig.IsAccountRunning(entry.userDataDir, out livePid) || !livePid.HasValue) return;
            uint targetPid = (uint)livePid.Value;
            NativeMethods.EnumWindows((hWnd, lParam) =>
            {
                uint pid;
                NativeMethods.GetWindowThreadProcessId(hWnd, out pid);
                if (pid == targetPid && NativeMethods.IsWindowVisible(hWnd))
                {
                    NativeMethods.SetForegroundWindow(hWnd);
                    return false;
                }
                return true;
            }, IntPtr.Zero);
        }

        public void ShowSettings()
        {
            using (var form = new SettingsForm())
            {
                form.ShowDialog(_mainWindow);
                RefreshAfterExternalChange();
            }
        }

        public void ShowAddAccountDialog()
        {
            using (var form = new AddAccountForm())
            {
                form.ShowDialog(_mainWindow);
                RefreshAfterExternalChange();
            }
        }

        // Exists purely to own a window handle for RegisterHotKey and receive
        // WM_HOTKEY messages - never shown. A hidden Form is used instead of a
        // bare NativeWindow.CreateHandle(new CreateParams()): the latter can
        // throw "Error creating window handle" (Win32Exception 0x80004005) in
        // practice, where Form's handle-creation path (exercised by every
        // WinForms app) does not.
        private sealed class MessageWindow : Form
        {
            public event Action<Message> HotkeyMessage;

            public MessageWindow()
            {
                ShowInTaskbar = false;
                FormBorderStyle = FormBorderStyle.FixedToolWindow;
                Size = new Size(0, 0);
            }

            public void EnsureHandleCreated()
            {
                var forceHandle = Handle; // creates the HWND without ever showing the form
            }

            protected override void SetVisibleCore(bool value)
            {
                base.SetVisibleCore(false); // never actually become visible
            }

            protected override void WndProc(ref Message m)
            {
                if (HotkeyMessage != null) HotkeyMessage(m);
                base.WndProc(ref m);
            }
        }
    }

    internal static class NativeMethods
    {
        public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

        [DllImport("user32.dll")]
        public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);

        [DllImport("user32.dll")]
        public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

        [DllImport("user32.dll")]
        public static extern bool IsWindowVisible(IntPtr hWnd);

        [DllImport("user32.dll")]
        public static extern bool SetForegroundWindow(IntPtr hWnd);
    }
}
