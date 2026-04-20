using System.Xml.Linq;
using ChickenScratch.Core.Compile;
using ChickenScratch.Core.Git;
using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;
using ChickenScratch.Core.Utils;

namespace ChickenScratch.Core.Scrivener;

public static class ScrivenerImporter
{
    public static Project Import(string scrivPath, string outputPath)
    {
        var scrivx = Directory.GetFiles(scrivPath, "*.scrivx").FirstOrDefault()
            ?? throw new InvalidOperationException("No .scrivx file found in Scrivener project.");

        var xdoc = XDocument.Load(scrivx);
        var binder = xdoc.Root?.Element("Binder")
            ?? throw new InvalidOperationException("Invalid .scrivx: no Binder element.");

        Directory.CreateDirectory(outputPath);
        Directory.CreateDirectory(Path.Combine(outputPath, "documents"));
        File.WriteAllText(Path.Combine(outputPath, ".gitignore"), ".project.yaml.tmp\n");

        var name = Path.GetFileNameWithoutExtension(scrivPath);
        var project = new Project
        {
            Id = Guid.NewGuid().ToString(),
            Name = name,
            Path = outputPath,
            Hierarchy =
            [
                new FolderNode { Id = "manuscript", Name = "Manuscript", Type = "folder" },
                new FolderNode { Id = "research",   Name = "Research",   Type = "folder" },
                new FolderNode { Id = "trash",       Name = "Trash",      Type = "folder" },
            ],
        };

        // First pass: build UUID → slug map
        var slugMap = new Dictionary<string, string>();
        BuildSlugMap(binder.Elements("BinderItem"), slugMap, project.Documents);

        // Second pass: convert items
        var manuscriptFolder = (FolderNode)project.Hierarchy[0];
        foreach (var item in binder.Elements("BinderItem"))
        {
            ConvertItem(item, scrivPath, outputPath, project, manuscriptFolder.Children, slugMap);
        }

        ProjectWriter.WriteProject(project);
        GitService.Init(outputPath);
        GitService.SaveRevision(outputPath, "Imported from Scrivener");
        return project;
    }

    private static void BuildSlugMap(IEnumerable<XElement> items, Dictionary<string, string> map, Dictionary<string, Document> docs)
    {
        foreach (var item in items)
        {
            var uuid = item.Attribute("UUID")?.Value ?? Guid.NewGuid().ToString();
            var title = item.Element("Title")?.Value ?? "Untitled";
            var slug = Slugify.UniqueSlug(title, string.Empty, docs);
            map[uuid] = slug;

            var children = item.Element("Children");
            if (children != null)
                BuildSlugMap(children.Elements("BinderItem"), map, docs);
        }
    }

    private static void ConvertItem(XElement item, string scrivPath, string outputPath,
        Project project, List<TreeNode> parent, Dictionary<string, string> slugMap)
    {
        var uuid = item.Attribute("UUID")?.Value ?? Guid.NewGuid().ToString();
        var type = item.Attribute("Type")?.Value ?? "Text";
        var title = item.Element("Title")?.Value ?? "Untitled";
        var id = Guid.NewGuid().ToString();

        if (type == "Text")
        {
            var rtfPath = Path.Combine(scrivPath, "Files", "Data", uuid, "content.rtf");
            var html = string.Empty;

            if (File.Exists(rtfPath))
            {
                try { html = PandocService.ConvertToHtml(rtfPath, "rtf"); }
                catch { html = string.Empty; }
            }

            html = CleanHtml(html);
            var slug = slugMap.TryGetValue(uuid, out var s) ? s : Slugify.Slugs(title);
            var relPath = $"documents/{slug}.html";

            var doc = new Document { Id = id, Name = title, Path = relPath, Content = html };
            project.Documents[id] = doc;
            ProjectWriter.WriteDocument(outputPath, doc);

            parent.Add(new DocumentNode { Id = id, Name = title, Path = relPath, Type = "document" });
        }
        else
        {
            var folder = new FolderNode { Id = id, Name = title, Type = "folder" };
            parent.Add(folder);

            var children = item.Element("Children");
            if (children != null)
            {
                foreach (var child in children.Elements("BinderItem"))
                    ConvertItem(child, scrivPath, outputPath, project, folder.Children, slugMap);
            }
        }
    }

    private static string CleanHtml(string html)
    {
        if (string.IsNullOrEmpty(html)) return html;

        // Remove scrivlnk:// links
        html = System.Text.RegularExpressions.Regex.Replace(
            html, @"href=""scrivlnk://[^""]*""", "");

        // Strip empty paragraphs introduced by pandoc
        html = System.Text.RegularExpressions.Regex.Replace(
            html, @"<p>\s*</p>", "");

        return html.Trim();
    }
}
