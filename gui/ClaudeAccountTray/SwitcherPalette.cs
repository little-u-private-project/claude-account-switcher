using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Windows.Forms;

namespace ClaudeAccountTray
{
    // The app's one persistent main window (there is no tray icon - closing
    // this exits the app): a styled, owner-drawn list of accounts with
    // status dots and hotkey chips, right-click actions per row, an "Add
    // account" call to action, and a Settings entry point - instead of a
    // bare default ListBox.
    public partial class SwitcherPalette : Form
    {
        private readonly TrayContext _tray;
        private readonly List<RowData> _rows = new List<RowData>();
        private readonly Timer _refreshTimer;
        private int _hoverIndex = -1;

        private class RowData
        {
            public string Email;
            public string DisplayName;
            public bool Running;
            public string Hotkey;
        }

        public SwitcherPalette(TrayContext tray)
        {
            _tray = tray;
            InitializeComponent();
            ApplyTheme();

            listAccounts.DrawItem += ListAccounts_DrawItem;
            listAccounts.MouseMove += ListAccounts_MouseMove;
            listAccounts.MouseLeave += (s, e) => { _hoverIndex = -1; listAccounts.Invalidate(); };
            listAccounts.MouseClick += ListAccounts_MouseClick;
            listAccounts.KeyDown += ListAccounts_KeyDown;
            listAccounts.DoubleClick += (s, e) => { if (_rows.Count == 0) OpenAddAccount(); else ActivateSelected(); };
            btnAddAccount.Click += (s, e) => OpenAddAccount();
            btnSettings.Click += (s, e) => _tray.ShowSettings();

            // Running status can change outside this app entirely (closing
            // Claude's own window, its tray "Exit", etc) - poll periodically
            // so the dot/status text don't go stale until the next click.
            _refreshTimer = new Timer();
            _refreshTimer.Interval = 5000;
            _refreshTimer.Tick += (s, e) => RefreshList();
            _refreshTimer.Start();
            FormClosed += (s, e) => _refreshTimer.Stop();
        }

        private void ApplyTheme()
        {
            BackColor = Theme.Background;
            listAccounts.BackColor = Theme.Background;
            lblTitle.Font = new Font("Segoe UI", 12F, FontStyle.Bold);
            lblTitle.ForeColor = Theme.TextPrimary;
            Theme.StyleAccentButton(btnAddAccount);
            Theme.StyleFlatButton(btnSettings);
        }

        // Lets external changes (Settings' Dark mode checkbox, row actions in
        // other dialogs) refresh both the theme and the row data on this
        // always-visible window immediately.
        public void ApplyThemeAndRedraw()
        {
            ApplyTheme();
            RefreshList();
        }

        public void RefreshList()
        {
            _rows.Clear();
            var accounts = InstanceConfig.GetKnownAccountEmails();
            var state = InstanceConfig.Load();
            // One WMI scan for the whole refresh, matched per account below,
            // instead of each account re-running its own full process-table
            // query every 5-second tick.
            var liveProcesses = InstanceConfig.GetRunningClaudeProcesses();
            bool stateChanged = false;
            foreach (var email in accounts)
            {
                var entry = InstanceConfig.Find(state, email);
                int? livePid;
                bool alive = InstanceConfig.Match(liveProcesses, entry != null ? entry.userDataDir : null, out livePid);
                if (entry != null && entry.pid != livePid)
                {
                    // Correct the recorded pid either way: clears a stale one
                    // (Claude closed outside this app's own Stop) and fixes a
                    // wrong one (e.g. MSIX handed off to a different process
                    // than the one we originally launched).
                    entry.pid = livePid;
                    stateChanged = true;
                }
                var row = new RowData();
                row.Email = email;
                row.DisplayName = InstanceConfig.DisplayName(entry, email);
                row.Running = alive;
                row.Hotkey = entry != null ? entry.hotkey : null;
                _rows.Add(row);
            }
            if (stateChanged) InstanceConfig.Save(state);

            // Preserve the current selection across a refresh (e.g. the
            // periodic timer tick) instead of always jumping back to row 0.
            string previouslySelected = null;
            int prevIdx = listAccounts.SelectedIndex;
            if (prevIdx >= 0 && prevIdx < _rows.Count) previouslySelected = _rows[prevIdx].Email;

            listAccounts.Items.Clear();
            if (_rows.Count == 0)
            {
                listAccounts.Items.Add("(empty)");
            }
            else
            {
                foreach (var r in _rows) listAccounts.Items.Add(r.Email);
            }
            _hoverIndex = -1;

            if (listAccounts.Items.Count == 0) return;
            int restoreIdx = 0;
            if (previouslySelected != null)
            {
                for (int i = 0; i < _rows.Count; i++)
                {
                    if (_rows[i].Email == previouslySelected) { restoreIdx = i; break; }
                }
            }
            listAccounts.SelectedIndex = restoreIdx;
        }

        private void OpenAddAccount()
        {
            _tray.ShowAddAccountDialog();
        }

        private void ListAccounts_MouseMove(object sender, MouseEventArgs e)
        {
            int idx = listAccounts.IndexFromPoint(e.Location);
            if (idx != _hoverIndex)
            {
                _hoverIndex = idx;
                listAccounts.Invalidate();
            }
        }

        private void ListAccounts_MouseClick(object sender, MouseEventArgs e)
        {
            int idx = listAccounts.IndexFromPoint(e.Location);
            if (idx < 0 || idx >= _rows.Count) return;
            listAccounts.SelectedIndex = idx;
            if (e.Button == MouseButtons.Right)
            {
                ShowRowMenu(_rows[idx], e.Location);
            }
        }

        private void ShowRowMenu(RowData row, Point at)
        {
            var menu = new ContextMenuStrip();
            Theme.StyleMenu(menu);

            var focusItem = new ToolStripMenuItem(row.Running ? "Focus" : "Launch");
            focusItem.Click += (s, e) => _tray.LaunchOrFocus(row.Email);
            menu.Items.Add(focusItem);

            if (row.Running)
            {
                var stopItem = new ToolStripMenuItem("Stop");
                stopItem.Click += (s, e) => { _tray.Stop(row.Email); RefreshList(); };
                menu.Items.Add(stopItem);
            }

            menu.Items.Add(new ToolStripSeparator());

            var hotkeyItem = new ToolStripMenuItem(
                string.IsNullOrEmpty(row.Hotkey) ? "Assign hotkey..." : "Change hotkey (" + row.Hotkey + ")...");
            hotkeyItem.Click += (s, e) => AssignHotkey(row);
            menu.Items.Add(hotkeyItem);

            if (!string.IsNullOrEmpty(row.Hotkey))
            {
                var removeHotkeyItem = new ToolStripMenuItem("Remove hotkey");
                removeHotkeyItem.Click += (s, e) => RemoveHotkey(row);
                menu.Items.Add(removeHotkeyItem);
            }

            var renameItem = new ToolStripMenuItem("Rename...");
            renameItem.Click += (s, e) => Rename(row);
            menu.Items.Add(renameItem);

            menu.Show(listAccounts, at);
        }

        private void AssignHotkey(RowData row)
        {
            using (var form = new HotkeyCaptureForm(row.Email, row.Hotkey))
            {
                if (form.ShowDialog(this) == DialogResult.OK)
                {
                    _tray.RefreshAfterExternalChange();
                    RefreshList();
                }
            }
        }

        private void Rename(RowData row)
        {
            using (var form = new RenameForm(row.Email, row.DisplayName == row.Email ? null : row.DisplayName))
            {
                if (form.ShowDialog(this) == DialogResult.OK)
                {
                    _tray.RefreshAfterExternalChange();
                    RefreshList();
                }
            }
        }

        private void RemoveHotkey(RowData row)
        {
            var entries = InstanceConfig.Load();
            var entry = InstanceConfig.Find(entries, row.Email);
            if (entry != null)
            {
                entry.hotkey = null;
                InstanceConfig.Save(entries);
                _tray.RefreshAfterExternalChange();
                RefreshList();
            }
        }

        private void ListAccounts_KeyDown(object sender, KeyEventArgs e)
        {
            if (e.KeyCode == Keys.Enter)
            {
                if (_rows.Count == 0) OpenAddAccount();
                else ActivateSelected();
                e.Handled = true;
            }
        }

        private void ActivateSelected()
        {
            int idx = listAccounts.SelectedIndex;
            if (idx < 0 || idx >= _rows.Count) return;
            var email = _rows[idx].Email;
            _tray.LaunchOrFocus(email);
        }

        private void ListAccounts_DrawItem(object sender, DrawItemEventArgs e)
        {
            if (e.Index < 0) return;
            e.DrawBackground();
            var g = e.Graphics;
            g.SmoothingMode = SmoothingMode.AntiAlias;
            var bounds = e.Bounds;

            bool hovered = e.Index == _hoverIndex;
            bool selected = (e.State & DrawItemState.Selected) == DrawItemState.Selected;
            var bg = selected ? Theme.RowSelected : (hovered ? Theme.RowHover : Theme.Background);
            using (var b = new SolidBrush(bg)) g.FillRectangle(b, bounds);

            if (_rows.Count == 0)
            {
                const string msg = "No saved accounts yet - click \"Add account\" below.";
                TextRenderer.DrawText(g, msg, Theme.FontRegular, bounds, Theme.TextSecondary,
                    TextFormatFlags.VerticalCenter | TextFormatFlags.HorizontalCenter | TextFormatFlags.WordBreak);
                return;
            }

            var row = _rows[e.Index];

            var dotColor = row.Running ? Theme.StatusRunning : Theme.StatusStopped;
            const int dotSize = 10;
            var dotRect = new Rectangle(bounds.Left + 14, bounds.Top + (bounds.Height - dotSize) / 2, dotSize, dotSize);
            using (var b = new SolidBrush(dotColor)) g.FillEllipse(b, dotRect);

            int textLeft = dotRect.Right + 14;
            var titleRect = new Rectangle(textLeft, bounds.Top + 6, bounds.Width - textLeft - 100, 20);
            TextRenderer.DrawText(g, row.DisplayName, Theme.FontBold, titleRect, Theme.TextPrimary,
                TextFormatFlags.VerticalCenter | TextFormatFlags.Left | TextFormatFlags.EndEllipsis);

            bool hasCustomName = row.DisplayName != row.Email;
            var subtitle = row.Running ? "Running" : "Not running";
            if (hasCustomName) subtitle = row.Email + "  ·  " + subtitle;
            var statusRect = new Rectangle(textLeft, bounds.Top + 26, bounds.Width - textLeft - 100, 18);
            TextRenderer.DrawText(g, subtitle, Theme.FontSmall, statusRect,
                Theme.TextSecondary, TextFormatFlags.VerticalCenter | TextFormatFlags.Left | TextFormatFlags.EndEllipsis);

            if (!string.IsNullOrEmpty(row.Hotkey))
            {
                var chipSize = TextRenderer.MeasureText(row.Hotkey, Theme.FontSmallBold);
                int chipW = chipSize.Width + 16;
                const int chipH = 22;
                var chipRect = new Rectangle(bounds.Right - chipW - 12, bounds.Top + (bounds.Height - chipH) / 2, chipW, chipH);
                using (var path = RoundedRect(chipRect, 6))
                using (var b = new SolidBrush(Theme.ChipBackground))
                {
                    g.FillPath(b, path);
                }
                TextRenderer.DrawText(g, row.Hotkey, Theme.FontSmallBold, chipRect, Theme.AccentDark,
                    TextFormatFlags.VerticalCenter | TextFormatFlags.HorizontalCenter);
            }
        }

        private static GraphicsPath RoundedRect(Rectangle bounds, int radius)
        {
            int d = radius * 2;
            var path = new GraphicsPath();
            path.AddArc(bounds.X, bounds.Y, d, d, 180, 90);
            path.AddArc(bounds.Right - d, bounds.Y, d, d, 270, 90);
            path.AddArc(bounds.Right - d, bounds.Bottom - d, d, d, 0, 90);
            path.AddArc(bounds.X, bounds.Bottom - d, d, d, 90, 90);
            path.CloseFigure();
            return path;
        }
    }
}
