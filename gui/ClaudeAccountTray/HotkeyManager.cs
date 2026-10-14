using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace ClaudeAccountTray
{
    // Thin P/Invoke wrapper around RegisterHotKey/UnregisterHotKey/WM_HOTKEY.
    // One registration per profile that has a hotkey assigned in instances.json.
    public class HotkeyManager
    {
        [DllImport("user32.dll")]
        private static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);
        [DllImport("user32.dll")]
        private static extern bool UnregisterHotKey(IntPtr hWnd, int id);

        private const uint MOD_ALT = 0x0001;
        private const uint MOD_CONTROL = 0x0002;
        private const uint MOD_SHIFT = 0x0004;
        private const uint MOD_WIN = 0x0008;
        private const int WM_HOTKEY = 0x0312;

        private readonly IntPtr _hWnd;
        private readonly Dictionary<int, string> _idToEmail = new Dictionary<int, string>();
        private int _nextId = 1;

        public event Action<string> HotkeyPressed;

        public HotkeyManager(IntPtr hWnd) { _hWnd = hWnd; }

        public void RegisterAll(IEnumerable<InstanceEntry> entries)
        {
            UnregisterAll();
            foreach (var e in entries)
            {
                if (string.IsNullOrWhiteSpace(e.hotkey)) continue;
                uint mods, vk;
                if (!TryParse(e.hotkey, out mods, out vk)) continue;
                int id = _nextId++;
                if (RegisterHotKey(_hWnd, id, mods, vk))
                {
                    _idToEmail[id] = e.email;
                }
            }
        }

        public void UnregisterAll()
        {
            foreach (var id in _idToEmail.Keys) UnregisterHotKey(_hWnd, id);
            _idToEmail.Clear();
        }

        public void OnWndProc(Message m)
        {
            if (m.Msg != WM_HOTKEY) return;
            int id = m.WParam.ToInt32();
            string email;
            if (_idToEmail.TryGetValue(id, out email))
            {
                if (HotkeyPressed != null) HotkeyPressed(email);
            }
        }

        // Parses a combo like "Ctrl+Alt+D1" (the exact text SettingsForm's
        // capture field produces) into RegisterHotKey's modifier bitmask + vk.
        public static bool TryParse(string combo, out uint modifiers, out uint vk)
        {
            modifiers = 0; vk = 0;
            if (string.IsNullOrWhiteSpace(combo)) return false;
            var parts = combo.Split('+');
            if (parts.Length < 2) return false;
            for (int i = 0; i < parts.Length - 1; i++)
            {
                switch (parts[i].Trim().ToLowerInvariant())
                {
                    case "ctrl":
                    case "control": modifiers |= MOD_CONTROL; break;
                    case "alt": modifiers |= MOD_ALT; break;
                    case "shift": modifiers |= MOD_SHIFT; break;
                    case "win":
                    case "windows": modifiers |= MOD_WIN; break;
                    default: return false;
                }
            }
            var keyName = parts[parts.Length - 1].Trim();
            try
            {
                var key = (Keys)Enum.Parse(typeof(Keys), keyName, true);
                vk = (uint)key;
                return true;
            }
            catch
            {
                return false;
            }
        }

        public enum CaptureResult { Ignored, Cleared, NoModifier, Captured }

        // Shared key-capture logic for the read-only hotkey textbox used by
        // both HotkeyCaptureForm and SettingsForm: builds a "Ctrl+Alt+D1"
        // combo string from a keydown, or reports why it couldn't yet
        // (nothing typed but a bare modifier, or no modifier held at all).
        public static CaptureResult CaptureKeyDown(KeyEventArgs e, out string combo)
        {
            combo = null;
            e.SuppressKeyPress = true;
            if (e.KeyCode == Keys.Back || e.KeyCode == Keys.Delete)
            {
                return CaptureResult.Cleared;
            }
            if (e.KeyCode == Keys.ControlKey || e.KeyCode == Keys.Menu ||
                e.KeyCode == Keys.ShiftKey || e.KeyCode == Keys.LWin || e.KeyCode == Keys.RWin)
            {
                return CaptureResult.Ignored; // a modifier alone isn't a complete combo yet
            }
            var parts = new List<string>();
            if (e.Control) parts.Add("Ctrl");
            if (e.Alt) parts.Add("Alt");
            if (e.Shift) parts.Add("Shift");
            if (parts.Count == 0)
            {
                return CaptureResult.NoModifier;
            }
            parts.Add(e.KeyCode.ToString());
            combo = string.Join("+", parts);
            return CaptureResult.Captured;
        }
    }
}
