using ChickenScratch.Core.Models;
using ChickenScratch.Core.Utils;

namespace ChickenScratch.Core.IO;

public static class DocumentService
{
    public static Project CreateDocument(string projectPath, string name, string? parentId = null)
    {
        var project = ProjectReader.ReadProject(projectPath);
        var slug = Slugify.UniqueSlug(name, Path.Combine(projectPath, "manuscript"), project.Documents);
        var relPath = $"manuscript/{slug}.md";
        var id = Guid.NewGuid().ToString();

        var doc = new Document { Id = id, Name = name, Path = relPath };
        project.Documents[id] = doc;

        var node = new DocumentNode { Id = id, Name = name, Path = relPath, Type = "document" };
        if (parentId != null)
            HierarchyOps.AddChildToFolder(project.Hierarchy, parentId, node);
        else
            HierarchyOps.AddToHierarchy(project.Hierarchy, node);

        ProjectWriter.WriteDocument(projectPath, doc);
        ProjectWriter.WriteProject(project);
        return project;
    }

    public static Project CreateFolder(string projectPath, string name, string? parentId = null)
    {
        var project = ProjectReader.ReadProject(projectPath);
        var id = Guid.NewGuid().ToString();
        var node = new FolderNode { Id = id, Name = name, Type = "folder" };

        if (parentId != null)
            HierarchyOps.AddChildToFolder(project.Hierarchy, parentId, node);
        else
            HierarchyOps.AddToHierarchy(project.Hierarchy, node);

        ProjectWriter.WriteProject(project);
        return project;
    }

    public static Project DeleteNode(string projectPath, string nodeId)
    {
        var project = ProjectReader.ReadProject(projectPath);
        var trash = HierarchyOps.FindFolder(project.Hierarchy, "Trash");

        // If already in trash, permanently delete
        if (trash != null && HierarchyOps.FindNode(trash.Children, nodeId) != null)
        {
            DeleteNodeFiles(nodeId, project, projectPath);
            HierarchyOps.RemoveNode(trash.Children, nodeId);
        }
        else
        {
            var node = HierarchyOps.RemoveNode(project.Hierarchy, nodeId);
            if (node != null && trash != null)
                trash.Children.Add(node);
        }

        ProjectWriter.WriteProject(project);
        return project;
    }

    public static Project MoveNode(string projectPath, string nodeId, string? newParentId, int? newIndex = null)
    {
        var project = ProjectReader.ReadProject(projectPath);
        HierarchyOps.MoveNode(project.Hierarchy, nodeId, newParentId);
        if (newIndex.HasValue)
            HierarchyOps.ReorderNode(project.Hierarchy, nodeId, newIndex.Value);
        ProjectWriter.WriteProject(project);
        return project;
    }

    public static Project RenameNode(string projectPath, string nodeId, string newName)
    {
        var project = ProjectReader.ReadProject(projectPath);
        var node = HierarchyOps.FindNode(project.Hierarchy, nodeId)
            ?? throw new InvalidOperationException($"Node {nodeId} not found");

        node.Name = newName;
        if (project.Documents.TryGetValue(nodeId, out var doc))
            doc.Name = newName;

        ProjectWriter.WriteProject(project);
        return project;
    }

    public static void UpdateContent(string projectPath, string docId, string htmlContent)
    {
        var project = ProjectReader.ReadProject(projectPath);
        if (!project.Documents.TryGetValue(docId, out var doc))
            throw new InvalidOperationException($"Document {docId} not found");

        doc.Content = htmlContent;
        doc.Modified = DateTime.UtcNow;
        ProjectWriter.WriteDocument(projectPath, doc);
    }

    public static Project UpdateMetadata(string projectPath, string docId, DocumentMetaUpdate update)
    {
        var project = ProjectReader.ReadProject(projectPath);
        if (!project.Documents.TryGetValue(docId, out var doc))
            throw new InvalidOperationException($"Document {docId} not found");

        if (update.Synopsis != null) doc.Synopsis = update.Synopsis;
        if (update.Label != null) doc.Label = update.Label;
        if (update.Status != null) doc.Status = update.Status;
        if (update.Keywords != null) doc.Keywords = update.Keywords;
        if (update.IncludeInCompile.HasValue) doc.IncludeInCompile = update.IncludeInCompile.Value;
        if (update.WordCountTarget.HasValue) doc.WordCountTarget = update.WordCountTarget.Value;
        doc.Modified = DateTime.UtcNow;

        ProjectWriter.WriteDocument(projectPath, doc);
        ProjectWriter.WriteProject(project);
        return project;
    }

    public static Project LinkDocuments(string projectPath, string docIdA, string docIdB)
    {
        var project = ProjectReader.ReadProject(projectPath);
        if (project.Documents.TryGetValue(docIdA, out var a) && !a.Links.Contains(docIdB))
            a.Links.Add(docIdB);
        if (project.Documents.TryGetValue(docIdB, out var b) && !b.Links.Contains(docIdA))
            b.Links.Add(docIdA);
        ProjectWriter.WriteProject(project);
        return project;
    }

    public static List<SearchResult> SearchProject(string projectPath, string query)
    {
        if (string.IsNullOrWhiteSpace(query)) return [];
        var project = ProjectReader.ReadProject(projectPath);
        var lower = query.ToLowerInvariant();
        var results = new List<SearchResult>();

        foreach (var doc in project.Documents.Values)
        {
            var text = System.Text.RegularExpressions.Regex.Replace(doc.Content, "<[^>]+>", " ");
            var textLower = text.ToLowerInvariant();
            var count = CountOccurrences(textLower, lower);
            if (count == 0) continue;

            var idx = textLower.IndexOf(lower);
            var start = Math.Max(0, idx - 60);
            var len = Math.Min(text.Length - start, 160);
            var snippet = (start > 0 ? "…" : "") + text.Substring(start, len).Trim() + (start + len < text.Length ? "…" : "");

            results.Add(new SearchResult { DocId = doc.Id, DocName = doc.Name, Snippet = snippet, MatchCount = count });
        }

        return [.. results.OrderByDescending(r => r.MatchCount)];
    }

    private static int CountOccurrences(string text, string pattern)
    {
        int count = 0, idx = 0;
        while ((idx = text.IndexOf(pattern, idx)) >= 0) { count++; idx += pattern.Length; }
        return count;
    }

    private static void DeleteNodeFiles(string nodeId, Project project, string projectPath)
    {
        if (project.Documents.TryGetValue(nodeId, out var doc))
        {
            var contentPath = Path.Combine(projectPath, doc.Path);
            if (File.Exists(contentPath)) File.Delete(contentPath);
            var metaPath = Path.ChangeExtension(contentPath, ".meta");
            if (File.Exists(metaPath)) File.Delete(metaPath);
            project.Documents.Remove(nodeId);
        }
    }
}
