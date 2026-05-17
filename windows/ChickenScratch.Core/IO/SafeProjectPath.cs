namespace ChickenScratch.Core.IO;

internal static class SafeProjectPath
{
    private static readonly char[] Separators = ['/', '\\'];

    public static void ValidateAllDocumentTargets(string projectPath, IEnumerable<string> documentPaths)
    {
        foreach (var documentPath in documentPaths)
        {
            var contentPath = GetDocumentContentPath(projectPath, documentPath, createParentDirectories: false);
            EnsureExistingPathSafe(contentPath, projectPath, documentPath, "document file");

            var metaPath = Path.ChangeExtension(contentPath, ".meta");
            EnsureExistingPathSafe(metaPath, projectPath, documentPath, "document metadata");
        }
    }

    public static string GetDocumentContentPath(
        string projectPath,
        string documentPath,
        bool createParentDirectories)
    {
        ValidateRelativeDocumentPath(documentPath);

        var projectRoot = GetProjectRoot(projectPath);
        var relativeParent = GetRelativeParent(documentPath);
        EnsureParentDirectorySafe(projectRoot, relativeParent, documentPath, createParentDirectories);

        var contentPath = Path.GetFullPath(Path.Combine(projectRoot, documentPath));
        EnsurePathWithinProject(projectRoot, contentPath, documentPath);
        return contentPath;
    }

    public static (string ContentPath, string MetaPath) GetExistingDocumentSidecarPaths(
        string projectPath,
        string documentPath)
    {
        var contentPath = GetDocumentContentPath(projectPath, documentPath, createParentDirectories: false);
        EnsureExistingPathSafe(contentPath, projectPath, documentPath, "document file");

        var metaPath = Path.ChangeExtension(contentPath, ".meta");
        EnsureExistingPathSafe(metaPath, projectPath, documentPath, "document metadata");

        return (contentPath, metaPath);
    }

    public static IEnumerable<string> EnumerateMarkdownFiles(string projectPath, string relativeRoot)
    {
        ValidateRelativeSubdirectoryPath(relativeRoot);

        var projectRoot = GetProjectRoot(projectPath);
        var rootPath = Path.GetFullPath(Path.Combine(projectRoot, relativeRoot));
        EnsurePathWithinProject(projectRoot, rootPath, relativeRoot);

        if (!Directory.Exists(rootPath)) yield break;

        EnsureDirectorySafe(rootPath, projectRoot, relativeRoot);

        foreach (var file in EnumerateMarkdownFilesRecursive(rootPath, projectRoot, relativeRoot))
            yield return file;
    }

    public static void EnsureExistingAbsoluteDocumentPathSafe(
        string projectPath,
        string absolutePath,
        string documentPath,
        string kind)
    {
        ValidateRelativeDocumentPath(documentPath);

        var projectRoot = GetProjectRoot(projectPath);
        var fullPath = Path.GetFullPath(absolutePath);
        EnsurePathWithinProject(projectRoot, fullPath, documentPath);
        EnsureExistingPathSafe(fullPath, projectRoot, documentPath, kind);
    }

    private static IEnumerable<string> EnumerateMarkdownFilesRecursive(
        string directoryPath,
        string projectRoot,
        string displayPath)
    {
        foreach (var entry in Directory.EnumerateFileSystemEntries(directoryPath))
        {
            var attributes = File.GetAttributes(entry);
            var entryDisplayPath = NormalizeDisplayPath(Path.GetRelativePath(projectRoot, entry));

            if ((attributes & FileAttributes.Directory) != 0)
            {
                EnsureDirectorySafe(entry, projectRoot, entryDisplayPath);
                foreach (var file in EnumerateMarkdownFilesRecursive(entry, projectRoot, entryDisplayPath))
                    yield return file;
                continue;
            }

            if (!entry.EndsWith(".md", StringComparison.OrdinalIgnoreCase)) continue;

            EnsureExistingPathSafe(entry, projectRoot, entryDisplayPath, "document file");
            yield return entry;
        }
    }

    private static void ValidateRelativeDocumentPath(string documentPath)
    {
        if (string.IsNullOrWhiteSpace(documentPath))
            throw InvalidDocumentPath(documentPath, "path must contain a file name");

        if (IsRooted(documentPath))
            throw InvalidDocumentPath(documentPath, "absolute paths are not allowed");

        var hasComponent = false;
        foreach (var component in documentPath.Split(Separators, StringSplitOptions.RemoveEmptyEntries))
        {
            if (component == ".")
                throw InvalidDocumentPath(documentPath, "current-directory components are not allowed");
            if (component == "..")
                throw InvalidDocumentPath(documentPath, "parent-directory components are not allowed");
            hasComponent = true;
        }

        if (!hasComponent)
            throw InvalidDocumentPath(documentPath, "path must contain a file name");
    }

    private static void ValidateRelativeSubdirectoryPath(string relativePath)
    {
        ValidateRelativeDocumentPath(relativePath);
        if (Path.GetFileName(relativePath).Contains('.', StringComparison.Ordinal))
            throw InvalidDocumentPath(relativePath, "project subdirectory path must not be a file");
    }

    private static string GetProjectRoot(string projectPath)
    {
        var projectRoot = Path.GetFullPath(projectPath);
        if (!Directory.Exists(projectRoot))
            throw new InvalidOperationException($"Project path is not a directory: {projectPath}");
        return TrimTrailingSeparator(projectRoot);
    }

    private static string GetRelativeParent(string documentPath)
    {
        var parts = documentPath.Split(Separators, StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length < 2)
            throw new InvalidOperationException($"Document has no parent: {documentPath}");
        return string.Join(Path.DirectorySeparatorChar, parts.Take(parts.Length - 1));
    }

    private static void EnsureParentDirectorySafe(
        string projectRoot,
        string relativeParent,
        string documentPath,
        bool createMissing)
    {
        var current = projectRoot;
        foreach (var component in relativeParent.Split(Separators, StringSplitOptions.RemoveEmptyEntries))
        {
            current = Path.Combine(current, component);
            EnsurePathWithinProject(projectRoot, current, documentPath);

            if (TryGetAttributes(current, out _))
            {
                EnsureDirectorySafe(current, projectRoot, documentPath);
                continue;
            }

            if (!createMissing) return;

            Directory.CreateDirectory(current);
            EnsureDirectorySafe(current, projectRoot, documentPath);
        }
    }

    private static void EnsureDirectorySafe(string path, string projectRoot, string documentPath)
    {
        var attributes = File.GetAttributes(path);
        if ((attributes & FileAttributes.ReparsePoint) != 0)
            throw new InvalidOperationException($"Document path traverses a symlink or reparse point: {documentPath} ({path})");
        if ((attributes & FileAttributes.Directory) == 0)
            throw new InvalidOperationException($"Document path parent is not a directory: {documentPath} ({path})");

        EnsurePathWithinProject(projectRoot, Path.GetFullPath(path), documentPath);
    }

    private static void EnsureExistingPathSafe(
        string path,
        string projectPathOrRoot,
        string documentPath,
        string kind)
    {
        if (!TryGetAttributes(path, out var attributes)) return;

        if ((attributes & FileAttributes.ReparsePoint) != 0)
            throw new InvalidOperationException($"Document {kind} is a symlink or reparse point: {documentPath} ({path})");

        var projectRoot = Directory.Exists(projectPathOrRoot)
            ? GetProjectRoot(projectPathOrRoot)
            : projectPathOrRoot;
        EnsurePathWithinProject(projectRoot, Path.GetFullPath(path), documentPath);
    }

    private static void EnsurePathWithinProject(string projectRoot, string fullPath, string documentPath)
    {
        var relative = Path.GetRelativePath(projectRoot, fullPath);
        if (relative == ".") return;
        if (relative == ".." ||
            relative.StartsWith(".." + Path.DirectorySeparatorChar, StringComparison.Ordinal) ||
            relative.StartsWith(".." + Path.AltDirectorySeparatorChar, StringComparison.Ordinal) ||
            IsRooted(relative))
        {
            throw InvalidDocumentPath(documentPath, "path escapes project root");
        }
    }

    private static bool IsRooted(string path) =>
        Path.IsPathRooted(path) ||
        path.StartsWith("\\", StringComparison.Ordinal) ||
        path.StartsWith("/", StringComparison.Ordinal) ||
        path.StartsWith("//", StringComparison.Ordinal) ||
        path.StartsWith(@"\\", StringComparison.Ordinal) ||
        (path.Length >= 2 && char.IsAsciiLetter(path[0]) && path[1] == ':');

    private static bool TryGetAttributes(string path, out FileAttributes attributes)
    {
        try
        {
            attributes = File.GetAttributes(path);
            return true;
        }
        catch (FileNotFoundException)
        {
            attributes = default;
            return false;
        }
        catch (DirectoryNotFoundException)
        {
            attributes = default;
            return false;
        }
    }

    private static InvalidOperationException InvalidDocumentPath(string? documentPath, string reason) =>
        new($"Document path must be relative and within project ({reason}): {documentPath}");

    private static string TrimTrailingSeparator(string path) =>
        path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);

    private static string NormalizeDisplayPath(string path) =>
        path.Replace('\\', '/');
}
