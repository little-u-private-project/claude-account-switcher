using System.Collections.Generic;
using System.Windows.Forms;

namespace ClaudeAccountTray
{
    // Scoped to exactly one thing: assign/clear a hotkey per saved profile.
    // Not a general preferences/config editor (see blueprint Out of scope).
    public partial class SettingsForm : Form
    {
        private readonly List<InstanceEntry> _entries;

        public SettingsForm()
        {
            InitializeComponent();
            ApplyTheme();
            _entries = InstanceConfig.Load();
            SeedMissingAccounts();
            RefreshList();

            listAccounts.SelectedIndexChanged += (s, e) => UpdateCaptureBox();
            txtHotkeyCapture.KeyDown += TxtHotkeyCapture_KeyDown;
            btnAssign.Click += (s, e) => AssignHotkey(txtHotkeyCapture.Text);
            btnClear.Click += (s, e) => AssignHotkey(null);
            btnClose.Click += (s, e) => { Save(); DialogResult = DialogResult.OK; Close(); };
            btnAddAccount.Click += (s, e) => ShowAddAccount();
            btnRename.Click += (s, e) => ShowRename();
            chkDarkMode.Checked = Theme.DarkMode;
            chkDarkMode.CheckedChanged += (s, e) =>
            {
                Theme.SetDarkMode(chkDarkMode.Checked);
                ApplyTheme();
                RefreshList();
            };
        }

        private void ApplyTheme()
        {
            BackColor = Theme.Card;
            listAccounts.BackColor = Theme.Card;
            listAccounts.Font = Theme.FontRegular;
            lblHotkey.Font = Theme.FontRegular;
            lblHotkey.ForeColor = Theme.TextPrimary;
            txtHotkeyCapture.Font = Theme.FontBold;
            Theme.StyleAccentButton(btnAddAccount);
            Theme.StyleFlatButton(btnRename);
            Theme.StyleAccentButton(btnAssign);
            Theme.StyleFlatButton(btnClear);
            Theme.StyleFlatButton(btnClose);
            chkDarkMode.ForeColor = Theme.TextPrimary;
        }

        // instances.json only gets an entry once claude-instance-start.ps1 has
        // run at least once for an account. Settings needs to offer every saved
        // account (from ~/.claude-accounts) even before its first launch.
        private void SeedMissingAccounts()
        {
            foreach (var email in InstanceConfig.GetKnownAccountEmails())
            {
                if (InstanceConfig.Find(_entries, email) == null)
                {
                    _entries.Add(new InstanceEntry { email = email });
                }
            }
        }

        private void RefreshList()
        {
            listAccounts.Items.Clear();
            if (_entries.Count == 0)
            {
                listAccounts.Items.Add("(no saved accounts yet - click \"Add account...\" below)");
                UpdateCaptureBox();
                return;
            }
            foreach (var e in _entries)
            {
                var label = InstanceConfig.DisplayName(e, e.email);
                if (!string.IsNullOrEmpty(e.hotkey)) label += "  [" + e.hotkey + "]";
                listAccounts.Items.Add(label);
            }
            if (listAccounts.Items.Count > 0) listAccounts.SelectedIndex = 0;
            UpdateCaptureBox();
        }

        private InstanceEntry SelectedEntry()
        {
            int i = listAccounts.SelectedIndex;
            return (i >= 0 && i < _entries.Count) ? _entries[i] : null;
        }

        private void UpdateCaptureBox()
        {
            var entry = SelectedEntry();
            txtHotkeyCapture.Text = (entry != null && entry.hotkey != null) ? entry.hotkey : "";
        }

        private void TxtHotkeyCapture_KeyDown(object sender, KeyEventArgs e)
        {
            string combo;
            var result = HotkeyManager.CaptureKeyDown(e, out combo);
            switch (result)
            {
                case HotkeyManager.CaptureResult.Cleared:
                    txtHotkeyCapture.Text = "";
                    break;
                case HotkeyManager.CaptureResult.NoModifier:
                    MessageBox.Show(this, "Hotkeys must include at least one modifier (Ctrl/Alt/Shift).", "Claude Account Switcher");
                    break;
                case HotkeyManager.CaptureResult.Captured:
                    txtHotkeyCapture.Text = combo;
                    break;
            }
        }

        private void AssignHotkey(string combo)
        {
            var entry = SelectedEntry();
            if (entry == null) return;
            uint parsedMods, parsedVk;
            if (!string.IsNullOrEmpty(combo) && !HotkeyManager.TryParse(combo, out parsedMods, out parsedVk))
            {
                MessageBox.Show(this, "Could not recognize that key combination.", "Claude Account Switcher");
                return;
            }
            entry.hotkey = string.IsNullOrEmpty(combo) ? null : combo;
            RefreshList();
        }

        private void Save()
        {
            InstanceConfig.Save(_entries);
        }

        private void ShowRename()
        {
            var entry = SelectedEntry();
            if (entry == null) return;
            using (var form = new RenameForm(entry.email, entry.displayName))
            {
                if (form.ShowDialog(this) == DialogResult.OK)
                {
                    var saved = InstanceConfig.Find(InstanceConfig.Load(), entry.email);
                    entry.displayName = saved != null ? saved.displayName : null;
                    RefreshList();
                }
            }
        }

        private void ShowAddAccount()
        {
            using (var form = new AddAccountForm())
            {
                form.ShowDialog(this);
                if (form.AnyAccountAdded)
                {
                    Save(); // persist any hotkeys assigned so far before reloading
                    _entries.Clear();
                    _entries.AddRange(InstanceConfig.Load());
                    SeedMissingAccounts();
                    RefreshList();
                }
            }
        }
    }
}
