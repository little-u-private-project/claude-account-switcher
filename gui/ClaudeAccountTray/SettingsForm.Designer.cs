namespace ClaudeAccountTray
{
    partial class SettingsForm
    {
        private System.ComponentModel.IContainer components = null;
        private System.Windows.Forms.ListBox listAccounts;
        private System.Windows.Forms.Label lblHotkey;
        private System.Windows.Forms.TextBox txtHotkeyCapture;
        private System.Windows.Forms.Button btnAssign;
        private System.Windows.Forms.Button btnClear;
        private System.Windows.Forms.Button btnClose;
        private System.Windows.Forms.Button btnAddAccount;
        private System.Windows.Forms.Button btnRename;
        private System.Windows.Forms.CheckBox chkDarkMode;

        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null)) components.Dispose();
            base.Dispose(disposing);
        }

        private void InitializeComponent()
        {
            this.listAccounts = new System.Windows.Forms.ListBox();
            this.lblHotkey = new System.Windows.Forms.Label();
            this.txtHotkeyCapture = new System.Windows.Forms.TextBox();
            this.btnAssign = new System.Windows.Forms.Button();
            this.btnClear = new System.Windows.Forms.Button();
            this.btnClose = new System.Windows.Forms.Button();
            this.btnAddAccount = new System.Windows.Forms.Button();
            this.btnRename = new System.Windows.Forms.Button();
            this.chkDarkMode = new System.Windows.Forms.CheckBox();
            this.SuspendLayout();

            this.listAccounts.Location = new System.Drawing.Point(12, 12);
            this.listAccounts.Size = new System.Drawing.Size(340, 120);
            this.listAccounts.Name = "listAccounts";

            this.btnAddAccount.Location = new System.Drawing.Point(12, 138);
            this.btnAddAccount.Size = new System.Drawing.Size(112, 27);
            this.btnAddAccount.Text = "Add account...";
            this.btnAddAccount.Name = "btnAddAccount";

            this.btnRename.Location = new System.Drawing.Point(130, 138);
            this.btnRename.Size = new System.Drawing.Size(112, 27);
            this.btnRename.Text = "Rename...";
            this.btnRename.Name = "btnRename";

            this.lblHotkey.Location = new System.Drawing.Point(12, 174);
            this.lblHotkey.Size = new System.Drawing.Size(120, 20);
            this.lblHotkey.Text = "Hotkey (press keys):";

            this.txtHotkeyCapture.Location = new System.Drawing.Point(140, 171);
            this.txtHotkeyCapture.Size = new System.Drawing.Size(212, 23);
            this.txtHotkeyCapture.ReadOnly = true;
            this.txtHotkeyCapture.Name = "txtHotkeyCapture";

            this.btnAssign.Location = new System.Drawing.Point(12, 202);
            this.btnAssign.Size = new System.Drawing.Size(90, 27);
            this.btnAssign.Text = "Assign";

            this.btnClear.Location = new System.Drawing.Point(108, 202);
            this.btnClear.Size = new System.Drawing.Size(90, 27);
            this.btnClear.Text = "Clear";

            this.chkDarkMode.Location = new System.Drawing.Point(12, 240);
            this.chkDarkMode.Size = new System.Drawing.Size(150, 24);
            this.chkDarkMode.Text = "Dark mode";
            this.chkDarkMode.Name = "chkDarkMode";

            this.btnClose.Location = new System.Drawing.Point(262, 238);
            this.btnClose.Size = new System.Drawing.Size(90, 27);
            this.btnClose.Text = "Close";

            this.AutoScaleDimensions = new System.Drawing.SizeF(96F, 96F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Dpi;
            this.ClientSize = new System.Drawing.Size(364, 276);
            this.Controls.Add(this.listAccounts);
            this.Controls.Add(this.btnAddAccount);
            this.Controls.Add(this.btnRename);
            this.Controls.Add(this.lblHotkey);
            this.Controls.Add(this.txtHotkeyCapture);
            this.Controls.Add(this.btnAssign);
            this.Controls.Add(this.btnClear);
            this.Controls.Add(this.chkDarkMode);
            this.Controls.Add(this.btnClose);
            this.FormBorderStyle = System.Windows.Forms.FormBorderStyle.FixedDialog;
            this.MaximizeBox = false;
            this.MinimizeBox = false;
            this.Name = "SettingsForm";
            this.StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen;
            this.Text = "Claude Account Switcher - Settings";
            this.ResumeLayout(false);
            this.PerformLayout();
        }
    }
}
