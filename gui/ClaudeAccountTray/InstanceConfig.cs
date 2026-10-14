using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Web.Script.Serialization;

namespace ClaudeAccountTray
{
    // One entry per saved Claude account. Mirrors exactly what
    // claude-instance-start.ps1 / claude-instance-stop.ps1 read and write to
    // the same file - this is the single shared state between the PowerShell
    // scripts and this GUI (no separate config/state split).
    public class InstanceEntry
    {
        public string email { get; set; }
        public string userDataDir { get; set; }
        public string hotkey { get; set; }
        public string displayName { get; set; }
        public int? pid { get; set; }
    }

    public static class InstanceConfig
    {
        private static readonly string UserProfile =
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);

        public static readonly string AccountsDir = Path.Combine(UserProfile, ".claude-accounts");
        public static readonly string StoreDir = Path.Combine(UserProfile, ".claude-instances");
        public static readonly string StateFile = Path.Combine(StoreDir, "instances.json");

        public static List<InstanceEntry> Load()
        {
            if (!File.Exists(StateFile)) return new List<InstanceEntry>();
            try
            {
                var json = File.ReadAllText(StateFile);
                if (string.IsNullOrWhiteSpace(json)) return new List<InstanceEntry>();
                var serializer = new JavaScriptSerializer();
                var list = serializer.Deserialize<List<InstanceEntry>>(json);
                return list ?? new List<InstanceEntry>();
            }
            catch
            {
                return new List<InstanceEntry>();
            }
        }

        public static void Save(List<InstanceEntry> entries)
        {
            Directory.CreateDirectory(StoreDir);
            var serializer = new JavaScriptSerializer();
            File.WriteAllText(StateFile, serializer.Serialize(entries));
        }

        public static InstanceEntry Find(List<InstanceEntry> entries, string email)
        {
            return entries.FirstOrDefault(e => string.Equals(e.email, email, StringComparison.OrdinalIgnoreCase));
        }

        // Finds the entry for email, creating (and appending) a bare one if
        // none exists yet - the same "load, find-or-create, set one field,
        // save" shape every settings-style dialog needs.
        public static InstanceEntry FindOrCreate(List<InstanceEntry> entries, string email)
        {
            var entry = Find(entries, email);
            if (entry == null)
            {
                entry = new InstanceEntry { email = email };
                entries.Add(entry);
            }
            return entry;
        }

        // The friendly label to show for an account: its custom display
        // name if one was set, otherwise the raw email.
        public static string DisplayName(InstanceEntry entry, string email)
        {
            if (entry != null && !string.IsNullOrEmpty(entry.displayName)) return entry.displayName;
            return email;
        }

        // Ground truth for "is this account's instance running": scan the
        // live OS process list for a Claude.exe whose command line was
        // launched with this account's isolated --user-data-dir folder,
        // rather than trusting a single pid recorded at launch time.
        //
        // A single captured pid can go wrong in ways that have nothing to do
        // with the account actually running or not: MSIX activation can hand
        // off from an initial process to a different one shortly after
        // launch, and a stored pid obviously can't survive this app's own
        // restart anyway. Re-deriving "running" from the live process table
        // every time is self-healing regardless of any of that - it doesn't
        // depend on anything this app remembered.
        //
        // Uses WMI (Win32_Process.CommandLine), not Process.MainModule:
        // Claude Desktop runs as an MSIX/Store app, and MainModule needs
        // PROCESS_VM_READ access that MSIX's AppContainer sandboxing denies
        // across the security boundary (that query throws and gets
        // swallowed, reporting every live process as not running). WMI reads
        // command lines from the OS process table instead and isn't subject
        // to that same per-handle access check for same-user processes.
        //
        // One query, reused across every account: a caller checking several
        // accounts (the main window's row list) should call
        // GetRunningClaudeProcesses() once and pass the result to Match() per
        // account, rather than each account re-running its own full WMI
        // process-table scan.
        public static List<KeyValuePair<int, string>> GetRunningClaudeProcesses()
        {
            var result = new List<KeyValuePair<int, string>>();
            try
            {
                using (var searcher = new System.Management.ManagementObjectSearcher(
                    "SELECT ProcessId, CommandLine FROM Win32_Process WHERE Name = 'Claude.exe'"))
                {
                    foreach (System.Management.ManagementObject mo in searcher.Get())
                    {
                        var cmdLine = mo["CommandLine"] as string;
                        if (string.IsNullOrEmpty(cmdLine)) continue;
                        result.Add(new KeyValuePair<int, string>(Convert.ToInt32(mo["ProcessId"]), cmdLine));
                    }
                }
            }
            catch
            {
                // Empty result -> every account reports "not running" - WMI
                // unavailable/blocked is rare, and a false "not running" is
                // the safer failure mode here (worst case: a redundant
                // launch, not a lost one).
            }
            return result;
        }

        public static bool Match(List<KeyValuePair<int, string>> liveProcesses, string userDataDir, out int? livePid)
        {
            livePid = null;
            if (string.IsNullOrEmpty(userDataDir) || liveProcesses == null) return false;
            foreach (var proc in liveProcesses)
            {
                if (proc.Value.IndexOf(userDataDir, StringComparison.OrdinalIgnoreCase) >= 0)
                {
                    livePid = proc.Key;
                    return true;
                }
            }
            return false;
        }

        // Convenience for a single-account check (one query for one
        // account). Prefer GetRunningClaudeProcesses()+Match() when checking
        // several accounts in the same operation.
        public static bool IsAccountRunning(string userDataDir, out int? livePid)
        {
            return Match(GetRunningClaudeProcesses(), userDataDir, out livePid);
        }

        // Saved accounts live in ~/.claude-accounts/<safe-email>/meta.json
        // regardless of whether concurrent mode has ever launched them -
        // that's claude-add.ps1's account store, shared for account discovery
        // per the blueprint (instances.json only gets an entry after first
        // launch, so it alone isn't enough to list "known" accounts).
        public static List<string> GetKnownAccountEmails()
        {
            var emails = new List<string>();
            if (!Directory.Exists(AccountsDir)) return emails;
            var serializer = new JavaScriptSerializer();
            foreach (var dir in Directory.GetDirectories(AccountsDir))
            {
                var meta = Path.Combine(dir, "meta.json");
                if (!File.Exists(meta)) continue;
                try
                {
                    var obj = serializer.Deserialize<Dictionary<string, object>>(File.ReadAllText(meta));
                    object email;
                    if (obj != null && obj.TryGetValue("email", out email) && email != null)
                    {
                        emails.Add(email.ToString());
                    }
                }
                catch { }
            }
            return emails;
        }
    }
}
