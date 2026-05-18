using ChickenScratch.Core.IO;
using ChickenScratch.Core.Models;

if (args.Length == 1 && args[0] == "--safe-paths")
{
    return RunSafePathHarness();
}

if (args.Length == 1 && args[0] == "--atomic-writes")
{
    return RunAtomicWritesHarness();
}

if (args.Length == 1 && args[0] == "--native-repair")
{
    return RunNativeRepairHarness();
}

if (args.Length != 1)
{
    Console.Error.WriteLine("usage: dotnet run --project windows/ChickenScratch.Core.Tests/CrossFrontendHarness <project.chikn>");
    Console.Error.WriteLine("       dotnet run --project windows/ChickenScratch.Core.Tests/CrossFrontendHarness -- --safe-paths");
    Console.Error.WriteLine("       dotnet run --project windows/ChickenScratch.Core.Tests/CrossFrontendHarness -- --atomic-writes");
    Console.Error.WriteLine("       dotnet run --project windows/ChickenScratch.Core.Tests/CrossFrontendHarness -- --native-repair");
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

static int RunAtomicWritesHarness()
{
    var tempRoot = Path.Combine(Path.GetTempPath(), "chickenscratch-atomic-writes-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(tempRoot);

    try
    {
        var projectPath = CreateProjectRoot(tempRoot, "atomic");
        var nestedDirectory = Path.Combine(projectPath, "manuscript", "nested");
        Directory.CreateDirectory(nestedDirectory);

        var existingContentPath = Path.Combine(nestedDirectory, "existing.md");
        var existingMetaPath = Path.Combine(nestedDirectory, "existing.meta");
        File.WriteAllText(existingContentPath, "old content");
        File.WriteAllText(existingMetaPath, "id: old\nname: Old\n");

        ProjectWriter.WriteDocument(projectPath, new Document
        {
            Id = "existing",
            Name = "Existing",
            ParentId = "manuscript",
            Path = "manuscript/nested/existing.md",
            Content = "new content\n",
            IncludeInCompile = false,
            Keywords = ["atomic", "replace"],
        });

        AssertEqual("new content\n", File.ReadAllText(existingContentPath), "existing .md should be replaced");
        AssertContains(File.ReadAllText(existingMetaPath), "id: existing", "existing .meta should be replaced");
        AssertNoAtomicTempFiles(nestedDirectory, "existing.md");
        AssertNoAtomicTempFiles(nestedDirectory, "existing.meta");

        var newContentPath = Path.Combine(projectPath, "manuscript", "created", "new.md");
        var newMetaPath = Path.Combine(projectPath, "manuscript", "created", "new.meta");
        ProjectWriter.WriteDocument(projectPath, new Document
        {
            Id = "new",
            Name = "New",
            ParentId = "manuscript",
            Path = "manuscript/created/new.md",
            Content = "created content\n",
        });

        AssertEqual("created content\n", File.ReadAllText(newContentPath), "new .md should be moved into place");
        AssertContains(File.ReadAllText(newMetaPath), "id: new", "new .meta should be moved into place");
        AssertNoAtomicTempFiles(Path.GetDirectoryName(newContentPath)!, "new.md");
        AssertNoAtomicTempFiles(Path.GetDirectoryName(newMetaPath)!, "new.meta");

        Console.WriteLine("ChickenScratch.Core.CrossFrontendHarness atomic-writes: passed");
        return 0;
    }
    finally
    {
        if (Directory.Exists(tempRoot))
            Directory.Delete(tempRoot, recursive: true);
    }
}

static int RunNativeRepairHarness()
{
    var tempRoot = Path.Combine(Path.GetTempPath(), "chickenscratch-native-repair-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(tempRoot);

    try
    {
        var missingBodyPath = CreateProjectRoot(tempRoot, "missing-body");
        WriteHierarchyProjectYaml(missingBodyPath, "missing-body-doc", "Missing Body", "manuscript/missing-body.md");

        var missingBodyProject = ProjectReader.ReadProject(missingBodyPath);
        AssertEqual(false, missingBodyProject.Documents.ContainsKey("missing-body-doc"), "missing body should not enter document map");
        ProjectWriter.WriteProject(missingBodyProject);
        var missingBodyYaml = File.ReadAllText(Path.Combine(missingBodyPath, "project.yaml"));
        AssertContains(missingBodyYaml, "id: missing-body-doc", "missing body hierarchy id should be preserved");
        AssertContains(missingBodyYaml, "path: manuscript/missing-body.md", "missing body hierarchy path should be preserved");
        AssertEqual(false, File.Exists(Path.Combine(missingBodyPath, "manuscript", "missing-body.md")), "missing body should not be recreated");

        var missingMetaPath = CreateProjectRoot(tempRoot, "missing-meta");
        File.WriteAllText(Path.Combine(missingMetaPath, "manuscript", "scene.md"), "scene body");
        WriteHierarchyProjectYaml(missingMetaPath, "scene-id", "Scene From Hierarchy", "manuscript/scene.md");

        var missingMetaProject = ProjectReader.ReadProject(missingMetaPath);
        AssertEqual(true, missingMetaProject.Documents.ContainsKey("scene-id"), "missing meta should recover hierarchy id");
        ProjectWriter.WriteProject(missingMetaProject);
        var recoveredMeta = File.ReadAllText(Path.Combine(missingMetaPath, "manuscript", "scene.meta"));
        AssertContains(recoveredMeta, "id: scene-id", "recovered meta should persist hierarchy id");
        AssertContains(recoveredMeta, "name: Scene From Hierarchy", "recovered meta should persist hierarchy name");

        var missingIDMetaPath = CreateProjectRoot(tempRoot, "missing-id-meta");
        File.WriteAllText(Path.Combine(missingIDMetaPath, "manuscript", "note.md"), "note body");
        File.WriteAllText(Path.Combine(missingIDMetaPath, "manuscript", "note.meta"), "name: Meta Name\nstatus: Draft\n");
        WriteHierarchyProjectYaml(missingIDMetaPath, "note-id", "Note From Hierarchy", "manuscript/note.md");

        var missingIDProject = ProjectReader.ReadProject(missingIDMetaPath);
        AssertEqual(true, missingIDProject.Documents.ContainsKey("note-id"), "meta without id should recover hierarchy id");
        AssertEqual("Draft", missingIDProject.Documents["note-id"].Status, "meta fields should survive id recovery");
        ProjectWriter.WriteProject(missingIDProject);
        var missingIDMeta = File.ReadAllText(Path.Combine(missingIDMetaPath, "manuscript", "note.meta"));
        AssertContains(missingIDMeta, "id: note-id", "rewritten meta should include recovered id");
        AssertContains(missingIDMeta, "status: Draft", "rewritten meta should preserve existing metadata");

        AssertThrows("corrupt meta should fail before native write", () =>
        {
            var corruptMetaPath = CreateProjectRoot(tempRoot, "corrupt-meta");
            File.WriteAllText(Path.Combine(corruptMetaPath, "manuscript", "bad.md"), "bad body");
            File.WriteAllText(Path.Combine(corruptMetaPath, "manuscript", "bad.meta"), "id: [unterminated\n");
            WriteHierarchyProjectYaml(corruptMetaPath, "bad-id", "Bad Meta", "manuscript/bad.md");
            _ = ProjectReader.ReadProject(corruptMetaPath);
        });

        AssertRejects("writer rejects hierarchy document without a path", () =>
        {
            var projectPath = CreateProjectRoot(tempRoot, "empty-hierarchy-path");
            var project = new Project
            {
                Id = Guid.NewGuid().ToString(),
                Name = "Empty Hierarchy Path",
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
                                Id = "empty-path",
                                Name = "Empty Path",
                                Path = "",
                                Type = "document",
                            },
                        ],
                    },
                ],
            };
            ProjectWriter.WriteProject(project);
        });

        Console.WriteLine("ChickenScratch.Core.CrossFrontendHarness native-repair: passed");
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

static void WriteHierarchyProjectYaml(string projectPath, string docId, string docName, string docPath)
{
    File.WriteAllText(
        Path.Combine(projectPath, "project.yaml"),
        $"""
        id: {Path.GetFileNameWithoutExtension(projectPath)}
        name: Native Repair
        created: 2026-01-01T00:00:00Z
        modified: 2026-01-01T00:00:00Z
        hierarchy:
        - id: manuscript
          name: Manuscript
          type: folder
          children:
          - id: {docId}
            name: {docName}
            type: document
            path: {docPath}
        - id: research
          name: Research
          type: folder
        - id: trash
          name: Trash
          type: folder
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

static void AssertThrows(string name, Action action)
{
    try
    {
        action();
    }
    catch (Exception)
    {
        return;
    }

    throw new InvalidOperationException($"{name}: expected an exception");
}

static void AssertEqual<T>(T expected, T actual, string message)
{
    if (!EqualityComparer<T>.Default.Equals(expected, actual))
        throw new InvalidOperationException($"{message}. Expected: {expected}; actual: {actual}");
}

static void AssertContains(string actual, string expectedSubstring, string message)
{
    if (!actual.Contains(expectedSubstring, StringComparison.Ordinal))
        throw new InvalidOperationException($"{message}. Expected to find: {expectedSubstring}; actual: {actual}");
}

static void AssertNoAtomicTempFiles(string directory, string finalFileName)
{
    var tempFiles = Directory.EnumerateFiles(directory, $".{finalFileName}.*.tmp").ToList();
    if (tempFiles.Count > 0)
        throw new InvalidOperationException($"atomic temp files were not cleaned up: {string.Join(", ", tempFiles)}");
}
