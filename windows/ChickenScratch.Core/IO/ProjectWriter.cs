using ChickenScratch.Core.Models;
using ChickenScratch.Core.Git;

namespace ChickenScratch.Core.IO;

public static class ProjectWriter
{
    public static void WriteProject(Project project)
    {
        project.Modified = DateTime.UtcNow;
        SafeProjectPath.ValidateAllDocumentTargets(project.Path, project.Documents.Values.Select(d => d.Path));

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
                SessionTarget = project.Metadata.SessionTarget is { IsEmpty: false } st
                    ? new SessionTargetYaml
                    {
                        WordsPerSession = st.WordsPerSession,
                        Deadline = string.IsNullOrEmpty(st.Deadline) ? null : st.Deadline,
                        TotalTarget = st.TotalTarget,
                    }
                    : null,
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

        WriteThreads(project);
    }

    public static void WriteDocument(string projectPath, Document doc)
    {
        var contentPath = SafeProjectPath.GetDocumentContentPath(
            projectPath,
            doc.Path,
            createParentDirectories: true);
        var (_, metaPath) = SafeProjectPath.GetExistingDocumentSidecarPaths(projectPath, doc.Path);

        var meta = new DocumentMetaYaml
        {
            // Identity — required for the Rust reader's `meta.id`-keyed
            // documents map. Omitting these caused F-001: hierarchy nodes
            // pointed to ids the cross-frontend reader couldn't find.
            Id = doc.Id,
            Name = doc.Name,
            ParentId = doc.ParentId,
            Created = doc.Created,
            Modified = doc.Modified,

            Synopsis = doc.Synopsis,
            Label = doc.Label,
            Status = doc.Status,
            Keywords = doc.Keywords.Count > 0 ? doc.Keywords : null,
            Links = doc.Links.Count > 0 ? doc.Links : null,
            // Canonical wire form is "Yes"/"No" strings, not a YAML bool.
            // The Rust reader (`Option<String>`) does not deserialize a bare
            // boolean into an option-of-string; older builds wrote `true`/`false`
            // here and the Tauri/Rust reader fell over on load (F-002).
            IncludeInCompile = doc.IncludeInCompile ? "Yes" : "No",
            WordCountTarget = doc.WordCountTarget,
            CompileOrder = doc.CompileOrder,
            SectionType = doc.SectionType,
            ScrivenerUuid = doc.ScrivenerUuid,
            Comments = doc.Comments.Count > 0
                ? doc.Comments.Select(c => new CommentYaml
                {
                    Id = c.Id,
                    Body = c.Body,
                    Resolved = c.Resolved,
                    Created = c.Created,
                    Modified = c.Modified,
                }).ToList()
                : null,
            Fields = doc.Fields.Count > 0 ? doc.Fields : null,
        };

        var metaYaml = YamlHelper.Serialize(meta);
        string? contentTempPath = null;
        string? metaTempPath = null;

        try
        {
            contentTempPath = WriteTempTextFile(contentPath, doc.Content);
            metaTempPath = WriteTempTextFile(metaPath, metaYaml);

            ReplaceOrMoveTempFile(contentTempPath, contentPath);
            contentTempPath = null;

            ReplaceOrMoveTempFile(metaTempPath, metaPath);
            metaTempPath = null;
        }
        finally
        {
            DeleteTempFileIfPresent(contentTempPath);
            DeleteTempFileIfPresent(metaTempPath);
        }
    }

    private static string WriteTempTextFile(string finalPath, string contents)
    {
        var directory = Path.GetDirectoryName(finalPath)
            ?? throw new InvalidOperationException($"Path has no parent directory: {finalPath}");
        var fileName = Path.GetFileName(finalPath);
        var tempPath = Path.Combine(directory, $".{fileName}.{Guid.NewGuid():N}.tmp");

        using (var stream = new FileStream(
            tempPath,
            FileMode.CreateNew,
            FileAccess.Write,
            FileShare.None,
            bufferSize: 4096,
            FileOptions.WriteThrough))
        using (var writer = new StreamWriter(stream, new System.Text.UTF8Encoding(encoderShouldEmitUTF8Identifier: false)))
        {
            writer.Write(contents);
        }

        return tempPath;
    }

    private static void ReplaceOrMoveTempFile(string tempPath, string finalPath)
    {
        if (File.Exists(finalPath))
        {
            File.Replace(tempPath, finalPath, destinationBackupFileName: null, ignoreMetadataErrors: true);
        }
        else
        {
            File.Move(tempPath, finalPath);
        }
    }

    private static void DeleteTempFileIfPresent(string? tempPath)
    {
        if (tempPath != null && File.Exists(tempPath))
            File.Delete(tempPath);
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

    /// <summary>
    /// Write `threads.yaml` at the project root, or remove it when the project
    /// has no threads. Removing rather than writing an empty file keeps clean
    /// projects free of clutter — matches `Writer.swift`/`writer.rs` behavior.
    /// </summary>
    private static void WriteThreads(Project project)
    {
        var path = Path.Combine(project.Path, "threads.yaml");
        if (project.Threads.Count == 0)
        {
            if (File.Exists(path)) File.Delete(path);
            return;
        }

        var payload = new ThreadsYamlRoot
        {
            Threads = project.Threads.Select(t => new ThreadYaml
            {
                Id = t.Id,
                Name = t.Name,
                Color = string.IsNullOrEmpty(t.Color) ? null : t.Color,
                Description = string.IsNullOrEmpty(t.Description) ? null : t.Description,
            }).ToList(),
        };
        File.WriteAllText(path, YamlHelper.Serialize(payload));
    }

    private static TreeNodeYaml SerializeNode(TreeNode node) => node switch
    {
        DocumentNode dn => new TreeNodeYaml { Id = dn.Id, Name = dn.Name, Type = "document", Path = dn.Path },
        FolderNode fn  => new TreeNodeYaml { Id = fn.Id, Name = fn.Name, Type = "folder",   Children = fn.Children.Select(SerializeNode).ToList() },
        _ => throw new InvalidOperationException($"Unknown node type: {node.GetType()}")
    };
}
