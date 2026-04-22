using System.Text.RegularExpressions;
using ChickenScratch.Core.Models;

namespace ChickenScratch.Core.Utils;

public static partial class Slugify
{
    [GeneratedRegex(@"[^a-z0-9]+")]
    private static partial Regex NonAlnum();

    [GeneratedRegex(@"-{2,}")]
    private static partial Regex MultiDash();

    public static string Slugs(string s)
    {
        var lower = s.ToLowerInvariant();
        var dashed = NonAlnum().Replace(lower, "-");
        var deduped = MultiDash().Replace(dashed, "-");
        return deduped.Trim('-');
    }

    public static string UniqueSlug(string name, string folder, Dictionary<string, Document> documents)
    {
        var base_ = Slugs(name);
        if (string.IsNullOrEmpty(base_)) base_ = "document";

        var candidate = base_;
        int n = 2;
        var usedPaths = documents.Values.Select(d => System.IO.Path.GetFileNameWithoutExtension(d.Path)).ToHashSet();

        while (usedPaths.Contains(candidate) || System.IO.File.Exists(System.IO.Path.Combine(folder, candidate + ".md")))
        {
            candidate = $"{base_}-{n++}";
        }

        return candidate;
    }
}
