namespace ClaudeAccountTray
{
    partial class AddAccountForm
    {
        private System.ComponentModel.IContainer components = null;
        private System.Windows.Forms.Label lblInstructions;
        private System.Windows.Forms.Label lblEmailCaption;
        private System.Windows.Forms.TextBox txtEmail;
        private System.Windows.Forms.Button btnAdd;
        private System.Windows.Forms.Button btnDone;
        private System.Windows.Forms.Button btnSwitchAccount;
        private System.Windows.Forms.Label lblStatus;

        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null)) components.Dispose();
            base.Dispose(disposing);
        }

        private void InitializeComponent()
        {
            this.lblInstructions = new System.Windows.Forms.Label();
            this.lblEmailCaption = new System.Windows.Forms.Label();
            this.txtEmail = new System.Windows.Forms.TextBox();
            this.btnAdd = new System.Windows.Forms.Button();
            this.btnDone = new System.Windows.Forms.Button();
            this.btnSwitchAccount = new System.Windows.Forms.Button();
            this.lblStatus = new System.Windows.Forms.Label();
            this.SuspendLayout();

            this.lblInstructions.Location = new System.Drawing.Point(12, 12);
            this.lblInstructions.Size = new System.Drawing.Size(360, 40);
            this.lblInstructions.Text = "Log into the account you want to add in the Claude Desktop app, then click Add.";

            this.lblEmailCaption.Location = new System.Drawing.Point(12, 58);
            this.lblEmailCaption.Size = new System.Drawing.Size(50, 23);
            this.lblEmailCaption.Text = "Email:";

            this.txtEmail.Location = new System.Drawing.Point(68, 55);
            this.txtEmail.Size = new System.Drawing.Size(304, 23);
            this.txtEmail.Name = "txtEmail";

            this.lblStatus.Location = new System.Drawing.Point(12, 85);
            this.lblStatus.Size = new System.Drawing.Size(360, 40);
            this.lblStatus.Name = "lblStatus";

            this.btnSwitchAccount.Location = new System.Drawing.Point(12, 130);
            this.btnSwitchAccount.Size = new System.Drawing.Size(198, 27);
            this.btnSwitchAccount.Text = "Log in as a different account";
            this.btnSwitchAccount.Name = "btnSwitchAccount";

            this.btnAdd.Location = new System.Drawing.Point(216, 130);
            this.btnAdd.Size = new System.Drawing.Size(75, 27);
            this.btnAdd.Text = "Add";
            this.btnAdd.UseVisualStyleBackColor = true;
            this.btnAdd.Name = "btnAdd";

            this.btnDone.Location = new System.Drawing.Point(297, 130);
            this.btnDone.Size = new System.Drawing.Size(75, 27);
            this.btnDone.Text = "Done";
            this.btnDone.UseVisualStyleBackColor = true;
            this.btnDone.Name = "btnDone";

            this.AutoScaleDimensions = new System.Drawing.SizeF(96F, 96F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Dpi;
            this.ClientSize = new System.Drawing.Size(384, 169);
            this.Controls.Add(this.lblInstructions);
            this.Controls.Add(this.lblEmailCaption);
            this.Controls.Add(this.txtEmail);
            this.Controls.Add(this.lblStatus);
            this.Controls.Add(this.btnSwitchAccount);
            this.Controls.Add(this.btnAdd);
            this.Controls.Add(this.btnDone);
            this.FormBorderStyle = System.Windows.Forms.FormBorderStyle.FixedDialog;
            this.MaximizeBox = false;
            this.MinimizeBox = false;
            this.Name = "AddAccountForm";
            this.StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen;
            this.Text = "Add Claude account";
            this.ResumeLayout(false);
            this.PerformLayout();
        }
    }
}
