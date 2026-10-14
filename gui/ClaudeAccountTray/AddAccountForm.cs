using System;
using System.ComponentModel;
using System.Drawing;
using System.IO;
using System.Windows.Forms;

namespace ClaudeAccountTray
{
    // Replaces claude-add.ps1's console Read-Host prompts with a proper
    // dialog: detect the logged-in email, let the user confirm/edit it,
    // save, and optionally chain straight to the next account - no
    // "go run this command yourself" step anywhere.
    public partial class AddAccountForm : Form
    {
        private static readonly string ToolsDir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".claude-tools");
        private static readonly string AddScript = Path.Combine(ToolsDir, "claude-add.ps1");

        public bool AnyAccountAdded { get; private set; }

        public AddAccountForm()
        {
            InitializeComponent();
            ApplyTheme();
            Load += (s, e) => DetectEmail();
            btnAdd.Click += (s, e) => AddAccount();
            btnSwitchAccount.Click += (s, e) => SwitchAccount();
            btnDone.Click += (s, e) =>
            {
                DialogResult = AnyAccountAdded ? DialogResult.OK : DialogResult.Cancel;
                Close();
            };
        }

        private void ApplyTheme()
        {
            BackColor = Theme.Card;
            lblInstructions.Font = Theme.FontRegular;
            lblInstructions.ForeColor = Theme.TextPrimary;
            lblEmailCaption.Font = Theme.FontRegular;
            lblEmailCaption.ForeColor = Theme.TextPrimary;
            txtEmail.Font = Theme.FontRegular;
            lblStatus.Font = Theme.FontSmall;
            Theme.StyleAccentButton(btnAdd);
            Theme.StyleFlatButton(btnDone);
            Theme.StyleFlatButton(btnSwitchAccount);
        }

        private void DetectEmail()
        {
            SetStatus("Checking which account is logged in...", false);
            SetBusy(true);

            var worker = new BackgroundWorker();
            worker.DoWork += (s, e) => { e.Result = PsRunner.RunCaptured(AddScript, "-DetectOnly"); };
            worker.RunWorkerCompleted += (s, e) =>
            {
                SetBusy(false);
                var result = e.Result as PsRunner.Result;
                var detected = result != null ? result.Output.Trim() : "";
                if (detected.Length > 0)
                {
                    txtEmail.Text = detected;
                    SetStatus("Detected: " + detected + " - click Add to save it.", false);
                }
                else
                {
                    SetStatus("Log into the account in Claude Desktop, then click Add.", false);
                }
            };
            worker.RunWorkerAsync();
        }

        private void AddAccount()
        {
            var email = txtEmail.Text.Trim();
            if (email.Length == 0)
            {
                SetStatus("Enter the account's email first.", true);
                return;
            }

            SetStatus("Saving " + email + " - Claude will close and reopen...", false);
            SetBusy(true);

            var worker = new BackgroundWorker();
            worker.DoWork += (s, e) => { e.Result = PsRunner.RunCaptured(AddScript, "-Email", email); };
            worker.RunWorkerCompleted += (s, e) =>
            {
                SetBusy(false);
                var result = e.Result as PsRunner.Result;
                if (result != null && result.ExitCode == 0)
                {
                    AnyAccountAdded = true;
                    SetStatus("Saved " + email + ". To add a different account, click "
                        + "\"Log in as a different account\" below. Otherwise, click Done.", false);
                }
                else
                {
                    var msg = (result != null && LastUsefulLine(result.Output) != null)
                        ? LastUsefulLine(result.Output)
                        : "Could not save the account.";
                    SetStatus(msg, true);
                }
            };
            worker.RunWorkerAsync();
        }

        // Clears the LOCAL Claude login only (never the app's real Log out,
        // which would revoke the session server-side and kill whatever was
        // just saved) and reopens Claude at a fresh sign-in screen, so a
        // second/third account can be logged into without ever touching the
        // app's own Log out button.
        private void SwitchAccount()
        {
            SetStatus("Signing out locally (not revoking any saved account) - Claude will reopen...", false);
            SetBusy(true);

            var worker = new BackgroundWorker();
            worker.DoWork += (s, e) => { e.Result = PsRunner.RunCaptured(AddScript, "-ClearLocalOnly"); };
            worker.RunWorkerCompleted += (s, e) =>
            {
                SetBusy(false);
                var result = e.Result as PsRunner.Result;
                if (result != null && result.ExitCode == 0)
                {
                    txtEmail.Text = "";
                    SetStatus("Log into the next account in Claude Desktop, then click Add.", false);
                }
                else
                {
                    var msg = (result != null && LastUsefulLine(result.Output) != null)
                        ? LastUsefulLine(result.Output)
                        : "Could not switch accounts.";
                    SetStatus(msg, true);
                }
            };
            worker.RunWorkerAsync();
        }

        // claude-add.ps1's Write-Host lines are logged newest-last; the most
        // relevant one for a failure is usually the final non-blank line.
        private static string LastUsefulLine(string output)
        {
            if (string.IsNullOrEmpty(output)) return null;
            var lines = output.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
            return lines.Length > 0 ? lines[lines.Length - 1].Trim() : null;
        }

        private void SetBusy(bool busy)
        {
            txtEmail.Enabled = !busy;
            btnAdd.Enabled = !busy;
            btnDone.Enabled = !busy;
            btnSwitchAccount.Enabled = !busy;
        }

        private void SetStatus(string text, bool isError)
        {
            lblStatus.Text = text;
            lblStatus.ForeColor = isError ? Color.Firebrick : Theme.TextSecondary;
        }

    }
}
