using System.Diagnostics;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;

namespace ChickenScratch.Core.Compile;

public static class PandocService
{
    public static string? FindPandoc()
    {
        var settings = SettingsService.GetSettings();
        if (!string.IsNullOrEmpty(settings.General.PandocPath) && TryPandoc(settings.General.PandocPath))
            return settings.General.PandocPath;

        foreach (var candidate in new[] { "pandoc", "pandoc.exe" })
        {
            if (TryPandoc(candidate)) return candidate;
        }
        return null;
    }

    public static string CheckPandoc()
    {
        var pandoc = FindPandoc()
            ?? throw new InvalidOperationException("Pandoc is not installed. Required for Scrivener import and manuscript export.");
        var result = Run(pandoc, "--version");
        return result.Split('\n')[0].Trim();
    }

    public static string ConvertToHtml(string inputPath, string format)
    {
        var pandoc = FindPandoc() ?? throw new InvalidOperationException("Pandoc not found");
        return Run(pandoc, $"-f {format} -t html --wrap=none \"{inputPath}\"");
    }

    public static void Compile(string projectPath, string outputPath, string format,
        string? title, string? author, CompileOptions? opts = null)
    {
        var pandoc = FindPandoc() ?? throw new InvalidOperationException("Pandoc not found");
        opts ??= new CompileOptions();
        var project = ProjectReader.ReadProject(projectPath);
        var docs = HierarchyOps.CollectManuscriptDocs(project.Hierarchy, project.Documents);

        if (docs.Count == 0)
            throw new InvalidOperationException("No documents in Manuscript to compile.");

        var parts = new List<string>();

        if (opts.IncludeTitlePage && (title != null || author != null))
        {
            var tp = "";
            if (title != null) tp += $"# {title}\n\n";
            if (author != null) tp += $"by {author}\n\n";
            tp += "---";
            parts.Add(tp);
        }

        var separator = string.IsNullOrEmpty(opts.SectionSeparator) ? "#" : opts.SectionSeparator;
        for (int i = 0; i < docs.Count; i++)
        {
            if (i > 0) parts.Add(separator);
            parts.Add(docs[i].Content ?? string.Empty);
        }

        var tempMd = Path.GetTempFileName() + ".md";

        try
        {
            File.WriteAllText(tempMd, string.Join("\n\n", parts));
            var metaArgs = "";
            if (title != null) metaArgs += $" --metadata title=\"{title}\"";
            if (author != null) metaArgs += $" --metadata author=\"{author}\"";
            Run(pandoc, $"-f gfm -t {format} -o \"{outputPath}\"{metaArgs} \"{tempMd}\"");
        }
        finally
        {
            if (File.Exists(tempMd)) File.Delete(tempMd);
        }
    }

    private static bool TryPandoc(string exe)
    {
        try
        {
            var p = Process.Start(new ProcessStartInfo(exe, "--version")
            {
                RedirectStandardOutput = true,
                UseShellExecute = false,
                CreateNoWindow = true,
            });
            p?.WaitForExit(3000);
            return p?.ExitCode == 0;
        }
        catch { return false; }
    }

    private static string Run(string exe, string args)
    {
        using var p = Process.Start(new ProcessStartInfo(exe, args)
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        }) ?? throw new InvalidOperationException($"Failed to start {exe}");

        var output = p.StandardOutput.ReadToEnd();
        var err = p.StandardError.ReadToEnd();
        p.WaitForExit();

        if (p.ExitCode != 0)
            throw new InvalidOperationException($"Pandoc failed: {err}");

        return output;
    }
}
