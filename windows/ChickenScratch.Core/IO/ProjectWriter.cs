using ChickenScratch.Core.Models;
using ChickenScratch.Core.Git;

namespace ChickenScratch.Core.IO;

public static class ProjectWriter
{
    public static void WriteProject(Project project)
    {
        project.Modified = DateTime.UtcNow;

        var root = new ProjectYamlRoot
        {
            Id = project.Id,
            Name = project.Name,
            Created = project.Created,
            Modified = project.Modified,
            Metadata = new ProjectMetaYaml
            {
                Title = project.Metadata.Title,
                Author = project.Metadata.Author,
                ProjectType = project.Metadata.ProjectType,
                Genre = project.Metadata.Genre,
                Theme = project.Metadata.Theme,
                Summary = project.Metadata.Summary,
            },
            Hierarchy = project.Hierarchy.Select(SerializeNode).ToList(),
        };

        var yaml = YamlHelper.Serialize(root);
        var tempPath = Path.Combine(project.Path, ".project.yaml.tmp");
        var finalPath = Path.Combine(project.Path, "project.yaml");

        File.WriteAllText(tempPath, yaml);
        File.Move(tempPath, finalPath, overwrite: true);

        // Write document files
        foreach (var doc in project.Documents.Values)
        {
            WriteDocument(project.Path, doc);
        }
    }

    public static void WriteDocument(string projectPath, Document doc)
    {
        var contentPath = Path.Combine(projectPath, doc.Path);
        Directory.CreateDirectory(Path.GetDirectoryName(contentPath)!);
        File.WriteAllText(contentPath, doc.Content);

        var meta = new DocumentMetaYaml
        {
            Synopsis = doc.Synopsis,
            Label = doc.Label,
            Status = doc.Status,
            Keywords = doc.Keywords.Count > 0 ? doc.Keywords : null,
            Links = doc.Links.Count > 0 ? doc.Links : null,
            IncludeInCompile = doc.IncludeInCompile,
            WordCountTarget = doc.WordCountTarget,
            CompileOrder = doc.CompileOrder,
            Created = doc.Created,
            Modified = doc.Modified,
        };

        var metaPath = Path.ChangeExtension(contentPath, ".meta");
        File.WriteAllText(metaPath, YamlHelper.Serialize(meta));
    }

    public static Project CreateProject(string projectPath, string name)
    {
        Directory.CreateDirectory(projectPath);
        Directory.CreateDirectory(Path.Combine(projectPath, "manuscript"));
        Directory.CreateDirectory(Path.Combine(projectPath, "research"));
        Directory.CreateDirectory(Path.Combine(projectPath, "trash"));

        File.WriteAllText(Path.Combine(projectPath, ".gitignore"), ".project.yaml.tmp\n");

        var project = new Project
        {
            Id = Guid.NewGuid().ToString(),
            Name = name,
            Path = projectPath,
            Hierarchy =
            [
                new FolderNode { Id = "manuscript", Name = "Manuscript", Type = "folder" },
                new FolderNode { Id = "research",   Name = "Research",   Type = "folder" },
                new FolderNode { Id = "trash",       Name = "Trash",      Type = "folder" },
            ],
        };

        WriteProject(project);
        GitService.Init(projectPath);
        GitService.SaveRevision(projectPath, $"Created project: {name}");
        return project;
    }

    private static TreeNodeYaml SerializeNode(TreeNode node) => node switch
    {
        DocumentNode dn => new TreeNodeYaml { Id = dn.Id, Name = dn.Name, Type = "document", Path = dn.Path },
        FolderNode fn  => new TreeNodeYaml { Id = fn.Id, Name = fn.Name, Type = "folder",   Children = fn.Children.Select(SerializeNode).ToList() },
        _ => throw new InvalidOperationException($"Unknown node type: {node.GetType()}")
    };
}

