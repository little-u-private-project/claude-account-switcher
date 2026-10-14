namespace ClaudeAccountTray
{
    partial class SwitcherPalette
    {
        private System.ComponentModel.IContainer components = null;
        private System.Windows.Forms.ListBox listAccounts;
        private System.Windows.Forms.Button btnAddAccount;
        private System.Windows.Forms.Button btnSettings;
        private System.Windows.Forms.Label lblTitle;

        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null)) components.Dispose();
            base.Dispose(disposing);
        }

        private void InitializeComponent()
        {
            this.listAccounts = new System.Windows.Forms.ListBox();
            this.btnAddAccount = new System.Windows.Forms.Button();
            this.btnSettings = new System.Windows.Forms.Button();
            this.lblTitle = new System.Windows.Forms.Label();
            this.SuspendLayout();

            this.lblTitle.Location = new System.Drawing.Point(16, 14);
            this.lblTitle.Size = new System.Drawing.Size(280, 24);
            this.lblTitle.Text = "Claude accounts";
            this.lblTitle.Name = "lblTitle";
            this.lblTitle.Anchor = System.Windows.Forms.AnchorStyles.Top | System.Windows.Forms.AnchorStyles.Left;

            this.btnSettings.Location = new System.Drawing.Point(324, 10);
            this.btnSettings.Size = new System.Drawing.Size(44, 30);
            this.btnSettings.Text = "⚙";
            this.btnSettings.Name = "btnSettings";
            this.btnSettings.Anchor = System.Windows.Forms.AnchorStyles.Top | System.Windows.Forms.AnchorStyles.Right;

            this.listAccounts.Location = new System.Drawing.Point(12, 44);
            this.listAccounts.Size = new System.Drawing.Size(356, 240);
            this.listAccounts.Name = "listAccounts";
            this.listAccounts.BorderStyle = System.Windows.Forms.BorderStyle.None;
            this.listAccounts.DrawMode = System.Windows.Forms.DrawMode.OwnerDrawFixed;
            this.listAccounts.ItemHeight = 48;
            this.listAccounts.IntegralHeight = false;
            this.listAccounts.Anchor = System.Windows.Forms.AnchorStyles.Top | System.Windows.Forms.AnchorStyles.Bottom
                | System.Windows.Forms.AnchorStyles.Left | System.Windows.Forms.AnchorStyles.Right;

            this.btnAddAccount.Location = new System.Drawing.Point(12, 292);
            this.btnAddAccount.Size = new System.Drawing.Size(356, 34);
            this.btnAddAccount.Text = "+  Add account";
            this.btnAddAccount.Name = "btnAddAccount";
            this.btnAddAccount.Anchor = System.Windows.Forms.AnchorStyles.Bottom | System.Windows.Forms.AnchorStyles.Left
                | System.Windows.Forms.AnchorStyles.Right;

            this.AutoScaleDimensions = new System.Drawing.SizeF(96F, 96F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Dpi;
            this.ClientSize = new System.Drawing.Size(380, 338);
            this.MinimumSize = new System.Drawing.Size(320, 260);
            this.Controls.Add(this.lblTitle);
            this.Controls.Add(this.btnSettings);
            this.Controls.Add(this.listAccounts);
            this.Controls.Add(this.btnAddAccount);
            this.FormBorderStyle = System.Windows.Forms.FormBorderStyle.Sizable;
            this.MaximizeBox = true;
            this.MinimizeBox = true;
            this.Name = "SwitcherPalette";
            this.ShowInTaskbar = true;
            this.StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen;
            this.Text = "Claude Accounts";
            this.ResumeLayout(false);
        }
    }
}
