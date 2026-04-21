using ChickenScratch.Core.Models;
using ChickenScratch.Core.Utils;

namespace ChickenScratch.Core.IO;

public static class ProjectReader
{
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
            }
        };

        project.Hierarchy = raw.Hierarchy.Select(n => ConvertNode(n)).ToList();

        // Load document content + meta
        CollectDocuments(project.Hierarchy, project.Documents, projectPath);

        // Repair: remove hierarchy entries whose .md files don't exist
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

    private static void CollectDocuments(List<TreeNode> nodes, Dictionary<string, Document> docs, string projectPath)
    {
        foreach (var node in nodes)
        {
            if (node is DocumentNode dn)
            {
                var contentPath = Path.Combine(projectPath, dn.Path);
                var content = File.Exists(contentPath) ? File.ReadAllText(contentPath) : string.Empty;

                var metaPath = Path.ChangeExtension(contentPath, ".meta");
                var meta = File.Exists(metaPath)
                    ? YamlHelper.Deserialize<DocumentMetaYaml>(File.ReadAllText(metaPath))
                    : new DocumentMetaYaml { Created = DateTime.UtcNow, Modified = DateTime.UtcNow };

                docs[dn.Id] = new Document
                {
                    Id = dn.Id,
                    Name = dn.Name,
                    Path = dn.Path,
                    Content = content,
                    Synopsis = meta.Synopsis,
                    Label = meta.Label,
                    Status = meta.Status,
                    Keywords = meta.Keywords ?? [],
                    Links = meta.Links ?? [],
                    IncludeInCompile = meta.IncludeInCompile,
                    WordCountTarget = meta.WordCountTarget,
                    CompileOrder = meta.CompileOrder,
                    Created = meta.Created == default ? DateTime.UtcNow : meta.Created,
                    Modified = meta.Modified == default ? DateTime.UtcNow : meta.Modified,
                };
            }
            else if (node is FolderNode folder)
            {
                CollectDocuments(folder.Children, docs, projectPath);
            }
        }
    }

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
