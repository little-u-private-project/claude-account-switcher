using System.Windows.Forms;

namespace ClaudeAccountTray
{
    // Sets a friendly display name for an account (shown instead of the raw
    // email in the palette/tray menu/Settings), stored in instances.json
    // alongside the hotkey - same pattern as HotkeyCaptureForm.
    public partial class RenameForm : Form
    {
        private readonly string _email;

        public RenameForm(string email, string currentDisplayName)
        {
            _email = email;
            InitializeComponent();
            ApplyTheme();

            lblCaption.Text = "Display name for " + email + ":";
            txtName.Text = string.IsNullOrEmpty(currentDisplayName) ? email : currentDisplayName;
            txtName.SelectAll();

            btnSave.Click += (s, e) => Save(txtName.Text.Trim());
            btnReset.Click += (s, e) => Save(null);
            btnCancel.Click += (s, e) => { DialogResult = DialogResult.Cancel; Close(); };
        }

        private void ApplyTheme()
        {
            BackColor = Theme.Card;
            lblCaption.Font = Theme.FontRegular;
            lblCaption.ForeColor = Theme.TextPrimary;
            txtName.Font = Theme.FontRegular;
            Theme.StyleAccentButton(btnSave);
            Theme.StyleFlatButton(btnReset);
            Theme.StyleFlatButton(btnCancel);
        }

        private void Save(string name)
        {
            var entries = InstanceConfig.Load();
            var entry = InstanceConfig.FindOrCreate(entries, _email);
            // Storing the email as the name is the same as clearing it - the
            // display falls back to the email anyway (InstanceConfig.DisplayName).
            entry.displayName = (string.IsNullOrEmpty(name) || name == _email) ? null : name;
            InstanceConfig.Save(entries);

            DialogResult = DialogResult.OK;
            Close();
        }
    }
}
