using System.Drawing;
using System.Windows.Forms;

namespace ClaudeAccountTray
{
    // Focused single-account hotkey assignment, opened from the palette's
    // right-click menu - saves straight to instances.json, no need to open
    // the full Settings list just to set one hotkey.
    public partial class HotkeyCaptureForm : Form
    {
        private readonly string _email;

        public HotkeyCaptureForm(string email, string currentHotkey)
        {
            _email = email;
            InitializeComponent();
            ApplyTheme();

            lblCaption.Text = "Hotkey for " + email + ":";
            txtCapture.Text = currentHotkey ?? "";

            txtCapture.KeyDown += TxtCapture_KeyDown;
            btnSave.Click += (s, e) => Save(txtCapture.Text);
            btnClear.Click += (s, e) => Save(null);
            btnCancel.Click += (s, e) => { DialogResult = DialogResult.Cancel; Close(); };
        }

        private void ApplyTheme()
        {
            BackColor = Theme.Card;
            lblCaption.Font = Theme.FontRegular;
            lblCaption.ForeColor = Theme.TextPrimary;
            txtCapture.Font = Theme.FontBold;
            Theme.StyleAccentButton(btnSave);
            Theme.StyleFlatButton(btnClear);
            Theme.StyleFlatButton(btnCancel);
        }

        private void TxtCapture_KeyDown(object sender, KeyEventArgs e)
        {
            string combo;
            var result = HotkeyManager.CaptureKeyDown(e, out combo);
            switch (result)
            {
                case HotkeyManager.CaptureResult.Cleared:
                    txtCapture.Text = "";
                    break;
                case HotkeyManager.CaptureResult.NoModifier:
                    MessageBox.Show(this, "Hotkeys must include at least one modifier (Ctrl/Alt/Shift).", "Claude Account Switcher");
                    break;
                case HotkeyManager.CaptureResult.Captured:
                    txtCapture.Text = combo;
                    break;
            }
        }

        private void Save(string combo)
        {
            if (!string.IsNullOrEmpty(combo))
            {
                uint m, v;
                if (!HotkeyManager.TryParse(combo, out m, out v))
                {
                    MessageBox.Show(this, "Could not recognize that key combination.", "Claude Account Switcher");
                    return;
                }
            }

            var entries = InstanceConfig.Load();
            var entry = InstanceConfig.FindOrCreate(entries, _email);
            entry.hotkey = string.IsNullOrEmpty(combo) ? null : combo;
            InstanceConfig.Save(entries);

            DialogResult = DialogResult.OK;
            Close();
        }
    }
}
