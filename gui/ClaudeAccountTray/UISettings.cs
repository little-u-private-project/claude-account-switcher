using System.IO;
using System.Web.Script.Serialization;

namespace ClaudeAccountTray
{
    // Small separate file from instances.json on purpose - this is a UI
    // preference (dark mode), not per-account state.
    public class UIPrefs
    {
        public bool darkMode { get; set; }
    }

    public static class UISettings
    {
        private static readonly string StateFile = Path.Combine(InstanceConfig.StoreDir, "ui-settings.json");

        public static UIPrefs Load()
        {
            if (!File.Exists(StateFile)) return new UIPrefs();
            try
            {
                var json = File.ReadAllText(StateFile);
                if (string.IsNullOrEmpty(json)) return new UIPrefs();
                var prefs = new JavaScriptSerializer().Deserialize<UIPrefs>(json);
                return prefs ?? new UIPrefs();
            }
            catch
            {
                return new UIPrefs();
            }
        }

        public static void Save(UIPrefs prefs)
        {
            Directory.CreateDirectory(InstanceConfig.StoreDir);
            File.WriteAllText(StateFile, new JavaScriptSerializer().Serialize(prefs));
        }
    }
}
