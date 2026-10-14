namespace ClaudeAccountTray
{
    partial class RenameForm
    {
        private System.ComponentModel.IContainer components = null;
        private System.Windows.Forms.Label lblCaption;
        private System.Windows.Forms.TextBox txtName;
        private System.Windows.Forms.Button btnSave;
        private System.Windows.Forms.Button btnReset;
        private System.Windows.Forms.Button btnCancel;

        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null)) components.Dispose();
            base.Dispose(disposing);
        }

        private void InitializeComponent()
        {
            this.lblCaption = new System.Windows.Forms.Label();
            this.txtName = new System.Windows.Forms.TextBox();
            this.btnSave = new System.Windows.Forms.Button();
            this.btnReset = new System.Windows.Forms.Button();
            this.btnCancel = new System.Windows.Forms.Button();
            this.SuspendLayout();

            this.lblCaption.Location = new System.Drawing.Point(14, 12);
            this.lblCaption.Size = new System.Drawing.Size(300, 40);
            this.lblCaption.Name = "lblCaption";

            this.txtName.Location = new System.Drawing.Point(14, 54);
            this.txtName.Size = new System.Drawing.Size(300, 25);
            this.txtName.Name = "txtName";

            this.btnSave.Location = new System.Drawing.Point(14, 92);
            this.btnSave.Size = new System.Drawing.Size(96, 28);
            this.btnSave.Text = "Save";
            this.btnSave.Name = "btnSave";

            this.btnReset.Location = new System.Drawing.Point(118, 92);
            this.btnReset.Size = new System.Drawing.Size(96, 28);
            this.btnReset.Text = "Use email";
            this.btnReset.Name = "btnReset";

            this.btnCancel.Location = new System.Drawing.Point(222, 92);
            this.btnCancel.Size = new System.Drawing.Size(92, 28);
            this.btnCancel.Text = "Cancel";
            this.btnCancel.Name = "btnCancel";

            this.AutoScaleDimensions = new System.Drawing.SizeF(96F, 96F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Dpi;
            this.ClientSize = new System.Drawing.Size(328, 134);
            this.Controls.Add(this.lblCaption);
            this.Controls.Add(this.txtName);
            this.Controls.Add(this.btnSave);
            this.Controls.Add(this.btnReset);
            this.Controls.Add(this.btnCancel);
            this.FormBorderStyle = System.Windows.Forms.FormBorderStyle.FixedDialog;
            this.MaximizeBox = false;
            this.MinimizeBox = false;
            this.ShowInTaskbar = false;
            this.Name = "RenameForm";
            this.StartPosition = System.Windows.Forms.FormStartPosition.CenterParent;
            this.Text = "Rename account";
            this.ResumeLayout(false);
            this.PerformLayout();
        }
    }
}
