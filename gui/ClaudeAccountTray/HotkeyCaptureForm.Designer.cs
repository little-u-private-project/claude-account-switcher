namespace ClaudeAccountTray
{
    partial class HotkeyCaptureForm
    {
        private System.ComponentModel.IContainer components = null;
        private System.Windows.Forms.Label lblCaption;
        private System.Windows.Forms.TextBox txtCapture;
        private System.Windows.Forms.Button btnSave;
        private System.Windows.Forms.Button btnClear;
        private System.Windows.Forms.Button btnCancel;

        protected override void Dispose(bool disposing)
        {
            if (disposing && (components != null)) components.Dispose();
            base.Dispose(disposing);
        }

        private void InitializeComponent()
        {
            this.lblCaption = new System.Windows.Forms.Label();
            this.txtCapture = new System.Windows.Forms.TextBox();
            this.btnSave = new System.Windows.Forms.Button();
            this.btnClear = new System.Windows.Forms.Button();
            this.btnCancel = new System.Windows.Forms.Button();
            this.SuspendLayout();

            this.lblCaption.Location = new System.Drawing.Point(14, 12);
            this.lblCaption.Size = new System.Drawing.Size(280, 40);
            this.lblCaption.Name = "lblCaption";

            this.txtCapture.Location = new System.Drawing.Point(14, 54);
            this.txtCapture.Size = new System.Drawing.Size(280, 25);
            this.txtCapture.ReadOnly = true;
            this.txtCapture.TextAlign = System.Windows.Forms.HorizontalAlignment.Center;
            this.txtCapture.Name = "txtCapture";

            this.btnSave.Location = new System.Drawing.Point(14, 92);
            this.btnSave.Size = new System.Drawing.Size(88, 28);
            this.btnSave.Text = "Save";
            this.btnSave.Name = "btnSave";

            this.btnClear.Location = new System.Drawing.Point(112, 92);
            this.btnClear.Size = new System.Drawing.Size(88, 28);
            this.btnClear.Text = "Clear";
            this.btnClear.Name = "btnClear";

            this.btnCancel.Location = new System.Drawing.Point(210, 92);
            this.btnCancel.Size = new System.Drawing.Size(84, 28);
            this.btnCancel.Text = "Cancel";
            this.btnCancel.Name = "btnCancel";

            this.AutoScaleDimensions = new System.Drawing.SizeF(96F, 96F);
            this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Dpi;
            this.ClientSize = new System.Drawing.Size(308, 134);
            this.Controls.Add(this.lblCaption);
            this.Controls.Add(this.txtCapture);
            this.Controls.Add(this.btnSave);
            this.Controls.Add(this.btnClear);
            this.Controls.Add(this.btnCancel);
            this.FormBorderStyle = System.Windows.Forms.FormBorderStyle.FixedDialog;
            this.MaximizeBox = false;
            this.MinimizeBox = false;
            this.ShowInTaskbar = false;
            this.Name = "HotkeyCaptureForm";
            this.StartPosition = System.Windows.Forms.FormStartPosition.CenterParent;
            this.Text = "Assign hotkey";
            this.ResumeLayout(false);
            this.PerformLayout();
        }
    }
}
