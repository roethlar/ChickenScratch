using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;

if (args.Length == 1 && args[0] == "--safe-paths")
{
    return RunSafePathHarness();
}

if (args.Length != 1)
{
    Console.Error.WriteLine("usage: dotnet run --project windows/ChickenScratch.Core.Tests/CrossFrontendHarness <project.chikn>");
    Console.Error.WriteLine("       dotnet run --project windows/ChickenScratch.Core.Tests/CrossFrontendHarness -- --safe-paths");
    return 1;
}

var projectPath = Path.GetFullPath(args[0]);
var project = ProjectReader.ReadProject(projectPath);
var doc = project.Documents.Values.OrderBy(d => d.Path, StringComparer.Ordinal).FirstOrDefault();

if (doc == null)
{
    Console.Error.WriteLine("ChickenScratch.Core.CrossFrontendHarness: project has no documents");
    return 1;
}

doc.Synopsis = "Cross-frontend harness: C# writer pass";
doc.Fields["cross_frontend_csharp"] = "ran";
doc.Fields["cross_frontend_sequence"] = new[] { "rust-converter", "swift-chiknkit", "csharp-core" };

ProjectWriter.WriteProject(project);

Console.WriteLine($"csharp: wrote {doc.Path} in {project.Path}");
return 0;

static int RunSafePathHarness()
{
    var tempRoot = Path.Combine(Path.GetTempPath(), "chickenscratch-safe-paths-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(tempRoot);

    try
    {
        AssertRejects("writer rejects rooted document path", () =>
        {
            var projectPath = CreateProjectRoot(tempRoot, "rooted");
            var rootedPath = OperatingSystem.IsWindows() ? @"C:\escape.md" : "/tmp/escape.md";
            var project = NewProject(projectPath, new Document
            {
                Id = "rooted",
                Name = "Rooted",
                Path = rootedPath,
                Content = "must not write",
            });
            ProjectWriter.WriteProject(project);
        });

        AssertRejects("writer rejects parent-directory component", () =>
        {
            var projectPath = CreateProjectRoot(tempRoot, "dotdot");
            var project = NewProject(projectPath, new Document
            {
                Id = "dotdot",
                Name = "DotDot",
                Path = "manuscript/../escape.md",
                Content = "must not write",
            });
            ProjectWriter.WriteProject(project);
        });

        AssertRejects("DocumentService rejects parent-directory component before delete", () =>
        {
            var projectPath = CreateProjectRoot(tempRoot, "delete-dotdot");
            File.WriteAllText(Path.Combine(projectPath, "manuscript", "victim.md"), "victim");
            File.WriteAllText(Path.Combine(projectPath, "manuscript", "victim.meta"), "id: victim\nname: Victim\n");
            File.WriteAllText(
                Path.Combine(projectPath, "project.yaml"),
                """
                id: delete-dotdot
                name: Delete DotDot
                created: 2026-01-01T00:00:00Z
                modified: 2026-01-01T00:00:00Z
                hierarchy:
                - id: trash
                  name: Trash
                  type: folder
                  children:
                  - id: victim
                    name: Victim
                    type: document
                    path: manuscript/../victim.md
                """);
            DocumentService.DeleteNode(projectPath, "victim");
        });

        RunSymlinkCases(tempRoot);

        Console.WriteLine("ChickenScratch.Core.CrossFrontendHarness safe-paths: passed");
        return 0;
    }
    finally
    {
        if (Directory.Exists(tempRoot))
            Directory.Delete(tempRoot, recursive: true);
    }
}

static void RunSymlinkCases(string tempRoot)
{
    if (!TryCreateSymlinkProbe(tempRoot))
    {
        Console.WriteLine("safe-paths: symlink cases skipped; platform does not allow symlink creation");
        return;
    }

    AssertRejects("writer rejects symlink parent", () =>
    {
        var projectPath = CreateProjectRoot(tempRoot, "symlink-parent");
        var outsidePath = Path.Combine(tempRoot, "outside-parent");
        Directory.CreateDirectory(outsidePath);
        Directory.CreateSymbolicLink(Path.Combine(projectPath, "manuscript", "link"), outsidePath);

        var project = NewProject(projectPath, new Document
        {
            Id = "linked-parent",
            Name = "Linked Parent",
            Path = "manuscript/link/pwned.md",
            Content = "must not escape",
        });
        ProjectWriter.WriteProject(project);
    });

    AssertRejects("writer rejects symlink .md target", () =>
    {
        var projectPath = CreateProjectRoot(tempRoot, "symlink-md");
        var outsideFile = Path.Combine(tempRoot, "outside.md");
        File.WriteAllText(outsideFile, "original");
        File.CreateSymbolicLink(Path.Combine(projectPath, "manuscript", "linked.md"), outsideFile);

        var project = NewProject(projectPath, new Document
        {
            Id = "linked-md",
            Name = "Linked Md",
            Path = "manuscript/linked.md",
            Content = "must not overwrite",
        });
        ProjectWriter.WriteProject(project);
    });

    AssertRejects("writer rejects symlink .meta target", () =>
    {
        var projectPath = CreateProjectRoot(tempRoot, "symlink-meta-write");
        var outsideMeta = Path.Combine(tempRoot, "outside-write.meta");
        File.WriteAllText(outsideMeta, "id: linked-meta\nname: Linked Meta\n");
        File.CreateSymbolicLink(Path.Combine(projectPath, "manuscript", "linked.meta"), outsideMeta);

        var project = NewProject(projectPath, new Document
        {
            Id = "linked-meta",
            Name = "Linked Meta",
            Path = "manuscript/linked.md",
            Content = "must not write metadata through link",
        });
        ProjectWriter.WriteProject(project);
    });

    AssertRejects("reader rejects symlink .meta target", () =>
    {
        var projectPath = CreateProjectRoot(tempRoot, "symlink-meta");
        File.WriteAllText(Path.Combine(projectPath, "manuscript", "doc.md"), "content");
        var outsideMeta = Path.Combine(tempRoot, "outside.meta");
        File.WriteAllText(outsideMeta, "id: doc\nname: Doc\n");
        File.CreateSymbolicLink(Path.Combine(projectPath, "manuscript", "doc.meta"), outsideMeta);
        WriteMinimalProjectYaml(projectPath);

        ProjectReader.ReadProject(projectPath);
    });
}

static bool TryCreateSymlinkProbe(string tempRoot)
{
    var target = Path.Combine(tempRoot, "symlink-probe-target");
    var link = Path.Combine(tempRoot, "symlink-probe-link");
    try
    {
        Directory.CreateDirectory(target);
        Directory.CreateSymbolicLink(link, target);
        return Directory.Exists(link);
    }
    catch
    {
        return false;
    }
    finally
    {
        if (Directory.Exists(link)) Directory.Delete(link);
        if (Directory.Exists(target)) Directory.Delete(target);
    }
}

static string CreateProjectRoot(string tempRoot, string name)
{
    var projectPath = Path.Combine(tempRoot, $"{name}.chikn");
    Directory.CreateDirectory(projectPath);
    Directory.CreateDirectory(Path.Combine(projectPath, "manuscript"));
    Directory.CreateDirectory(Path.Combine(projectPath, "research"));
    Directory.CreateDirectory(Path.Combine(projectPath, "trash"));
    return projectPath;
}

static Project NewProject(string projectPath, Document doc)
{
    return new Project
    {
        Id = Guid.NewGuid().ToString(),
        Name = Path.GetFileNameWithoutExtension(projectPath),
        Path = projectPath,
        Hierarchy =
        [
            new FolderNode
            {
                Id = "manuscript",
                Name = "Manuscript",
                Type = "folder",
                Children =
                [
                    new DocumentNode
                    {
                        Id = doc.Id,
                        Name = doc.Name,
                        Path = doc.Path,
                        Type = "document",
                    }
                ],
            },
            new FolderNode { Id = "research", Name = "Research", Type = "folder" },
            new FolderNode { Id = "trash", Name = "Trash", Type = "folder" },
        ],
        Documents = new Dictionary<string, Document> { [doc.Id] = doc },
    };
}

static void WriteMinimalProjectYaml(string projectPath)
{
    File.WriteAllText(
        Path.Combine(projectPath, "project.yaml"),
        """
        id: safe-path-reader
        name: Safe Path Reader
        created: 2026-01-01T00:00:00Z
        modified: 2026-01-01T00:00:00Z
        hierarchy: []
        """);
}

static void AssertRejects(string name, Action action)
{
    try
    {
        action();
    }
    catch (InvalidOperationException)
    {
        return;
    }

    throw new InvalidOperationException($"{name}: expected InvalidOperationException");
}
