using System;
using System.Windows.Forms;

namespace ClaudeAccountTray
{
    internal static class Program
    {
        [STAThread]
        private static void Main()
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            try
            {
                Application.Run(new TrayContext());
            }
            catch (Exception ex)
            {
                MessageBox.Show(ex.ToString(), "Claude Account Tray - startup error");
            }
        }
    }
}
