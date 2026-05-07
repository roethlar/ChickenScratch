using ChickenScratch.Core.Models;
using ChickenScratch.Core.Utils;
using YamlDotNet.RepresentationModel;

// Disambiguate from `System.Threading.Thread` (pulled in by ImplicitUsings).
using PlotThread = ChickenScratch.Core.Models.Thread;

namespace ChickenScratch.Core.IO;

public static class ProjectReader
{
    /// <summary>
    /// Folders walked for `.md` documents. Mirrors `read_all_documents` in
    /// `crates/core/src/core/project/reader.rs`. `characters/` and `locations/`
    /// hold first-class entity documents that the format intentionally keeps
    /// out of `project.yaml.hierarchy`.
    /// </summary>
    private static readonly string[] DocumentRoots =
    [
        "manuscript", "research", "templates", "characters", "locations",
    ];

    public static Project ReadProject(string projectPath)
    {
        var yamlPath = Path.Combine(projectPath, "project.yaml");
        if (!File.Exists(yamlPath))
            throw new InvalidOperationException($"Not a valid .chikn project: {projectPath}");

        var raw = YamlHelper.Deserialize<ProjectYamlRoot>(File.ReadAllText(yamlPath));

        var project = new Project
        {
            Id = raw.Id,
            Name = raw.Name,
            Path = projectPath,
            Created = raw.Created,
            Modified = raw.Modified,
            Metadata = raw.Metadata == null ? new() : new ProjectMetadata
            {
                Title = raw.Metadata.Title,
                Author = raw.Metadata.Author,
                ProjectType = raw.Metadata.ProjectType,
                Genre = raw.Metadata.Genre,
                Theme = raw.Metadata.Theme,
                Summary = raw.Metadata.Summary,
                SessionTarget = raw.Metadata.SessionTarget is { } st && !IsEmpty(st)
                    ? new SessionTarget
                    {
                        WordsPerSession = st.WordsPerSession,
                        Deadline = st.Deadline,
                        TotalTarget = st.TotalTarget,
                    }
                    : null,
            }
        };

        project.Hierarchy = raw.Hierarchy.Select(n => ConvertNode(n)).ToList();

        // Walk disk for all .md files, not just hierarchy entries. Entity
        // documents (characters/, locations/) are intentionally absent from
        // `project.yaml.hierarchy`; without this walk Windows could not see
        // them and a save would orphan them on disk (F-004).
        CollectAllDocuments(project, projectPath);

        // Load threads.yaml sidecar when present. Missing/empty file → empty list.
        project.Threads = ReadThreads(projectPath);

        // Repair: remove hierarchy entries whose .md files don't exist.
        // Documents loaded from disk but missing from hierarchy stay in
        // `project.Documents` (matches the Rust reader's orphan handling for
        // entity folders that live outside hierarchy).
        RepairHierarchy(project.Hierarchy, project.Documents, projectPath);

        // Repair: ensure standard top-level folders exist
        EnsureStandardFolders(project);

        return project;
    }

    private static TreeNode ConvertNode(TreeNodeYaml yaml)
    {
        if (yaml.Type.Equals("folder", StringComparison.OrdinalIgnoreCase))
        {
            return new FolderNode
            {
                Id = yaml.Id,
                Name = yaml.Name,
                Type = "folder",
                Children = yaml.Children?.Select(c => ConvertNode(c)).ToList() ?? []
            };
        }

        return new DocumentNode
        {
            Id = yaml.Id,
            Name = yaml.Name,
            Type = "document",
            Path = yaml.Path ?? string.Empty,
        };
    }

    private static void CollectAllDocuments(Project project, string projectPath)
    {
        // Index document nodes by their hierarchy id so we can promote names
        // and ids when meta is missing/synthetic.
        var hierarchyByPath = new Dictionary<string, DocumentNode>(StringComparer.OrdinalIgnoreCase);
        IndexHierarchy(project.Hierarchy, hierarchyByPath);

        foreach (var root in DocumentRoots)
        {
            var rootPath = Path.Combine(projectPath, root);
            if (!Directory.Exists(rootPath)) continue;

            foreach (var mdFile in Directory.EnumerateFiles(rootPath, "*.md", SearchOption.AllDirectories))
            {
                var doc = ReadDocument(mdFile, projectPath, hierarchyByPath);
                if (doc != null)
                    project.Documents[doc.Id] = doc;
            }
        }
    }

    private static void IndexHierarchy(List<TreeNode> nodes, Dictionary<string, DocumentNode> byPath)
    {
        foreach (var node in nodes)
        {
            if (node is DocumentNode dn && !string.IsNullOrEmpty(dn.Path))
                byPath[NormalizePath(dn.Path)] = dn;
            else if (node is FolderNode folder)
                IndexHierarchy(folder.Children, byPath);
        }
    }

    private static Document? ReadDocument(
        string mdAbsolutePath,
        string projectPath,
        Dictionary<string, DocumentNode> hierarchyByPath)
    {
        var relPath = NormalizePath(System.IO.Path.GetRelativePath(projectPath, mdAbsolutePath));
        var content = File.Exists(mdAbsolutePath) ? File.ReadAllText(mdAbsolutePath) : string.Empty;

        var metaPath = System.IO.Path.ChangeExtension(mdAbsolutePath, ".meta");
        DocumentMetaYaml? meta = null;
        bool? legacyIncludeBool = null;

        if (File.Exists(metaPath))
        {
            var metaText = File.ReadAllText(metaPath);
            meta = YamlHelper.Deserialize<DocumentMetaYaml>(metaText);
            // YamlDotNet won't deserialize a YAML bool into a string property,
            // so older Windows-written `.meta` files (which used a bool here)
            // arrive with `IncludeInCompile == null`. Re-parse the raw YAML
            // node and recover the bool when present (F-002 read-side legacy).
            legacyIncludeBool = ParseLegacyIncludeInCompileBool(metaText);
        }

        // Fall back to the hierarchy entry only when we have no meta at all.
        // The Rust reader treats meta as authoritative; we do the same so a
        // Windows save can't introduce a divergent id.
        hierarchyByPath.TryGetValue(relPath, out var hierarchyNode);

        var id = !string.IsNullOrEmpty(meta?.Id)
            ? meta!.Id!
            : hierarchyNode?.Id ?? Guid.NewGuid().ToString();

        var name = !string.IsNullOrEmpty(meta?.Name)
            ? meta!.Name!
            : hierarchyNode?.Name ?? System.IO.Path.GetFileNameWithoutExtension(mdAbsolutePath);

        var includeInCompile = DecodeIncludeInCompile(meta?.IncludeInCompile, legacyIncludeBool);

        var comments = meta?.Comments?.Select(c => new Comment
        {
            Id = c.Id,
            Body = c.Body,
            Resolved = c.Resolved,
            Created = c.Created,
            Modified = c.Modified,
        }).ToList() ?? [];

        return new Document
        {
            Id = id,
            Name = name,
            Path = relPath,
            Content = content,
            ParentId = meta?.ParentId,
            Synopsis = meta?.Synopsis,
            Label = meta?.Label,
            Status = meta?.Status,
            Keywords = meta?.Keywords ?? [],
            Links = meta?.Links ?? [],
            IncludeInCompile = includeInCompile,
            WordCountTarget = meta?.WordCountTarget ?? 0,
            CompileOrder = meta?.CompileOrder ?? 0,
            SectionType = meta?.SectionType,
            ScrivenerUuid = meta?.ScrivenerUuid,
            Comments = comments,
            Created = meta?.Created is { } c && c != default ? c : DateTime.UtcNow,
            Modified = meta?.Modified is { } m && m != default ? m : DateTime.UtcNow,
            Fields = meta?.Fields ?? [],
        };
    }

    /// <summary>
    /// Resolve `include_in_compile` from any of the wire forms the format has
    /// produced over time. Default is `true` (include) when no field is present.
    /// </summary>
    internal static bool DecodeIncludeInCompile(string? str, bool? legacyBool)
    {
        if (!string.IsNullOrEmpty(str))
        {
            // Canonical "Yes"/"No"; anything else (including unknown strings)
            // matches the Rust reader's "treat anything not == 'No' as included".
            return !str.Equals("No", StringComparison.OrdinalIgnoreCase);
        }
        if (legacyBool.HasValue) return legacyBool.Value;
        return true;
    }

    /// <summary>
    /// Recover a YAML boolean for `include_in_compile` from an older Windows
    /// `.meta` file. Returns null when the field is absent or already a string.
    /// </summary>
    private static bool? ParseLegacyIncludeInCompileBool(string yamlText)
    {
        try
        {
            var stream = new YamlStream();
            stream.Load(new StringReader(yamlText));
            if (stream.Documents.Count == 0) return null;

            if (stream.Documents[0].RootNode is not YamlMappingNode root) return null;

            foreach (var entry in root.Children)
            {
                if (entry.Key is YamlScalarNode key &&
                    key.Value == "include_in_compile" &&
                    entry.Value is YamlScalarNode val)
                {
                    if (val.Style is YamlDotNet.Core.ScalarStyle.SingleQuoted
                        or YamlDotNet.Core.ScalarStyle.DoubleQuoted)
                    {
                        // Quoted scalar — treat as string, no legacy bool.
                        return null;
                    }
                    if (bool.TryParse(val.Value, out var b)) return b;
                    return null;
                }
            }
        }
        catch
        {
            // Malformed YAML is the deserializer's problem; we only opportunistically
            // recover the legacy bool form here.
        }
        return null;
    }

    private static List<PlotThread> ReadThreads(string projectPath)
    {
        var path = Path.Combine(projectPath, "threads.yaml");
        if (!File.Exists(path)) return [];
        var text = File.ReadAllText(path);
        if (string.IsNullOrWhiteSpace(text)) return [];

        var parsed = YamlHelper.Deserialize<ThreadsYamlRoot>(text);
        return parsed.Threads
            .Where(t => !string.IsNullOrEmpty(t.Id) && !string.IsNullOrEmpty(t.Name))
            .Select(t => new PlotThread
            {
                Id = t.Id,
                Name = t.Name,
                Color = string.IsNullOrEmpty(t.Color) ? null : t.Color,
                Description = string.IsNullOrEmpty(t.Description) ? null : t.Description,
            })
            .ToList();
    }

    private static bool IsEmpty(SessionTargetYaml st) =>
        st.WordsPerSession == null && string.IsNullOrEmpty(st.Deadline) && st.TotalTarget == null;

    private static string NormalizePath(string path) =>
        path.Replace('\\', '/');

    private static void RepairHierarchy(List<TreeNode> nodes, Dictionary<string, Document> docs, string projectPath)
    {
        for (int i = nodes.Count - 1; i >= 0; i--)
        {
            if (nodes[i] is DocumentNode dn)
            {
                if (!File.Exists(Path.Combine(projectPath, dn.Path)))
                {
                    docs.Remove(dn.Id);
                    nodes.RemoveAt(i);
                }
            }
            else if (nodes[i] is FolderNode folder)
            {
                RepairHierarchy(folder.Children, docs, projectPath);
            }
        }
    }

    private static void EnsureStandardFolders(Project project)
    {
        var names = new[] { "Manuscript", "Research", "Trash" };
        foreach (var name in names)
        {
            if (HierarchyOps.FindFolder(project.Hierarchy, name) == null)
            {
                project.Hierarchy.Add(new FolderNode
                {
                    Id = name.ToLowerInvariant(),
                    Name = name,
                    Type = "folder",
                });
            }
        }
    }
}
