using System.Diagnostics;
using System.Text;

namespace ClaudeAccountTray
{
    // Centralizes "run one of the vendored .ps1 scripts" - correct Windows
    // argv escaping, standard flags, optional output capture - so callers
    // can't each construct the command line by hand and get the quoting
    // wrong (see EscapeArg).
    internal static class PsRunner
    {
        public class Result
        {
            public string Output;
            public int ExitCode;
        }

        // Fire-and-forget: waits for the script but doesn't capture output.
        public static void RunFireAndForget(string scriptPath, params string[] namedArgs)
        {
            RunCore(scriptPath, namedArgs, false, 30000);
        }

        // Captures combined stdout+stderr and the exit code, for callers
        // that need to know success/failure or parse printed output (e.g.
        // claude-add.ps1 -DetectOnly's printed email).
        public static Result RunCaptured(string scriptPath, params string[] namedArgs)
        {
            return RunCore(scriptPath, namedArgs, true, 60000);
        }

        private static Result RunCore(string scriptPath, string[] namedArgs, bool captureOutput, int timeoutMs)
        {
            var sb = new StringBuilder();
            sb.Append("-NoProfile -ExecutionPolicy Bypass -File ").Append(EscapeArg(scriptPath));
            foreach (var a in namedArgs)
            {
                sb.Append(' ').Append(EscapeArg(a));
            }

            var psi = new ProcessStartInfo
            {
                FileName = "powershell.exe",
                Arguments = sb.ToString(),
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = captureOutput,
                RedirectStandardError = captureOutput
            };

            using (var p = Process.Start(psi))
            {
                string output = null;
                if (captureOutput)
                {
                    output = p.StandardOutput.ReadToEnd() + p.StandardError.ReadToEnd();
                }
                p.WaitForExit(timeoutMs);
                var result = new Result();
                result.Output = output;
                result.ExitCode = p.ExitCode;
                return result;
            }
        }

        // Windows CommandLineToArgvW-compatible quoting: a backslash is
        // literal unless it immediately precedes a quote, in which case
        // every backslash in that run must be doubled and the quote itself
        // escaped. Building this properly (rather than naive
        // "\"" + value + "\"" concatenation) is what actually prevents a
        // value containing a `"` - an account email, ultimately sourced
        // from a network response, is exactly such a value - from breaking
        // out of its argument and injecting a sibling powershell.exe switch
        // (e.g. -Command) into the process being launched.
        public static string EscapeArg(string arg)
        {
            if (string.IsNullOrEmpty(arg)) return "\"\"";
            var sb = new StringBuilder();
            sb.Append('"');
            int backslashes = 0;
            foreach (char c in arg)
            {
                if (c == '\\') { backslashes++; continue; }
                if (c == '"')
                {
                    sb.Append('\\', backslashes * 2 + 1);
                    sb.Append('"');
                    backslashes = 0;
                    continue;
                }
                if (backslashes > 0) { sb.Append('\\', backslashes); backslashes = 0; }
                sb.Append(c);
            }
            if (backslashes > 0) sb.Append('\\', backslashes * 2);
            sb.Append('"');
            return sb.ToString();
        }
    }
}
