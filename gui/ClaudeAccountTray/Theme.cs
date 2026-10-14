using System.Drawing;
using System.Windows.Forms;

namespace ClaudeAccountTray
{
    // One shared palette/fonts so every window (palette, settings, add
    // account) looks like the same app instead of default-gray WinForms.
    // Colors are computed properties (not static readonly fields) so
    // toggling DarkMode and re-applying a form's theme picks up the new
    // values immediately, no restart needed.
    internal static class Theme
    {
        public static bool DarkMode = UISettings.Load().darkMode;

        public static void SetDarkMode(bool value)
        {
            DarkMode = value;
            var prefs = new UIPrefs();
            prefs.darkMode = value;
            UISettings.Save(prefs);
        }

        public static Color Background
        {
            get { return DarkMode ? Color.FromArgb(32, 30, 28) : Color.FromArgb(250, 249, 247); }
        }
        public static Color Card
        {
            get { return DarkMode ? Color.FromArgb(42, 39, 36) : Color.White; }
        }
        public static Color RowHover
        {
            get { return DarkMode ? Color.FromArgb(58, 53, 49) : Color.FromArgb(241, 233, 226); }
        }
        public static Color RowSelected
        {
            get { return DarkMode ? Color.FromArgb(95, 58, 42) : Color.FromArgb(250, 224, 209); }
        }
        public static Color Accent
        {
            get { return DarkMode ? Color.FromArgb(224, 134, 104) : Color.FromArgb(203, 108, 78); }
        }
        public static Color AccentDark
        {
            get { return DarkMode ? Color.FromArgb(240, 160, 132) : Color.FromArgb(171, 85, 58); }
        }
        public static Color TextPrimary
        {
            get { return DarkMode ? Color.FromArgb(238, 233, 228) : Color.FromArgb(38, 33, 29); }
        }
        public static Color TextSecondary
        {
            get { return DarkMode ? Color.FromArgb(173, 163, 153) : Color.FromArgb(133, 122, 112); }
        }
        public static Color Border
        {
            get { return DarkMode ? Color.FromArgb(66, 61, 56) : Color.FromArgb(228, 220, 212); }
        }
        public static Color StatusRunning
        {
            get { return DarkMode ? Color.FromArgb(74, 199, 120) : Color.FromArgb(45, 164, 89); }
        }
        public static Color StatusStopped
        {
            get { return DarkMode ? Color.FromArgb(122, 113, 104) : Color.FromArgb(176, 167, 158); }
        }
        public static Color ChipBackground
        {
            get { return DarkMode ? Color.FromArgb(74, 56, 46) : Color.FromArgb(240, 228, 219); }
        }

        public static readonly Font FontRegular = new Font("Segoe UI", 9.75F);
        public static readonly Font FontBold = new Font("Segoe UI", 9.75F, FontStyle.Bold);
        public static readonly Font FontSmall = new Font("Segoe UI", 8.25F);
        public static readonly Font FontSmallBold = new Font("Segoe UI", 8.25F, FontStyle.Bold);

        // A flat accent-colored button (the "+ Add account" call to action).
        public static void StyleAccentButton(Button b)
        {
            b.FlatStyle = FlatStyle.Flat;
            b.FlatAppearance.BorderSize = 0;
            b.BackColor = Accent;
            b.ForeColor = Color.White;
            b.Font = FontBold;
            b.Cursor = Cursors.Hand;
            b.FlatAppearance.MouseOverBackColor = AccentDark;
            b.FlatAppearance.MouseDownBackColor = AccentDark;
        }

        // A flat neutral button (secondary actions: Clear, Close, Cancel...).
        public static void StyleFlatButton(Button b)
        {
            b.FlatStyle = FlatStyle.Flat;
            b.FlatAppearance.BorderColor = Border;
            b.BackColor = Card;
            b.ForeColor = TextPrimary;
            b.Font = FontRegular;
            b.Cursor = Cursors.Hand;
            b.FlatAppearance.MouseOverBackColor = RowHover;
        }

        // Right-click/tray menus otherwise render as default Windows gray -
        // this makes them match the rest of the app, including dark mode.
        private sealed class ThemeColorTable : ProfessionalColorTable
        {
            public override Color MenuItemSelected { get { return RowHover; } }
            public override Color MenuItemSelectedGradientBegin { get { return RowHover; } }
            public override Color MenuItemSelectedGradientEnd { get { return RowHover; } }
            public override Color MenuItemBorder { get { return Accent; } }
            public override Color MenuBorder { get { return Border; } }
            public override Color ToolStripDropDownBackground { get { return Card; } }
            public override Color ImageMarginGradientBegin { get { return Card; } }
            public override Color ImageMarginGradientMiddle { get { return Card; } }
            public override Color ImageMarginGradientEnd { get { return Card; } }
        }

        public static readonly ToolStripProfessionalRenderer MenuRenderer =
            new ToolStripProfessionalRenderer(new ThemeColorTable());

        public static void StyleMenu(ContextMenuStrip menu)
        {
            menu.Renderer = MenuRenderer;
            menu.Font = FontRegular;
            menu.ForeColor = TextPrimary;
            menu.BackColor = Card;
        }
    }
}
