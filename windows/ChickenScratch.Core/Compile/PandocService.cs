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
            var tp = "<div class='title-page'>";
            if (title != null) tp += $"<h1>{System.Net.WebUtility.HtmlEncode(title)}</h1>";
            if (author != null) tp += $"<p class='author'>{System.Net.WebUtility.HtmlEncode(author)}</p>";
            tp += "</div>";
            parts.Add(tp);
        }

        for (int i = 0; i < docs.Count; i++)
        {
            if (i > 0) parts.Add($"<p class='separator'>{System.Net.WebUtility.HtmlEncode(opts.SectionSeparator)}</p>");
            parts.Add(docs[i].Content);
        }

        var font = opts.Font ?? "Times New Roman";
        var fontSize = opts.FontSize ?? 12f;
        var lineSpacing = opts.LineSpacing ?? 2f;
        var margin = opts.MarginInches ?? 1f;

        var css = $@"<style>
body {{ font-family: '{font}'; font-size: {fontSize}pt; line-height: {lineSpacing}; margin: {margin}in; }}
.title-page {{ text-align: center; margin-bottom: 2in; }}
.separator {{ text-align: center; margin: 1em 0; }}
</style>";

        var html = $"<html><head>{css}</head><body>{string.Join("\n", parts)}</body></html>";
        var tempHtml = Path.GetTempFileName() + ".html";

        try
        {
            File.WriteAllText(tempHtml, html);
            var metaArgs = "";
            if (title != null) metaArgs += $" --metadata title=\"{title}\"";
            if (author != null) metaArgs += $" --metadata author=\"{author}\"";
            Run(pandoc, $"-f html -t {format} -o \"{outputPath}\"{metaArgs} \"{tempHtml}\"");
        }
        finally
        {
            if (File.Exists(tempHtml)) File.Delete(tempHtml);
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
