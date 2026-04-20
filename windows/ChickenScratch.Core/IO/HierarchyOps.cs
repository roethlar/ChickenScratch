using ChickenScratch.Core.Models;

namespace ChickenScratch.Core.IO;

public static class HierarchyOps
{
    public static void AddToHierarchy(List<TreeNode> hierarchy, TreeNode node)
        => hierarchy.Add(node);

    public static bool AddChildToFolder(List<TreeNode> hierarchy, string parentId, TreeNode node)
    {
        foreach (var item in hierarchy)
        {
            if (item is FolderNode folder)
            {
                if (folder.Id == parentId)
                {
                    folder.Children.Add(node);
                    return true;
                }
                if (AddChildToFolder(folder.Children, parentId, node))
                    return true;
            }
        }
        return false;
    }

    public static TreeNode? RemoveNode(List<TreeNode> hierarchy, string nodeId)
    {
        for (int i = 0; i < hierarchy.Count; i++)
        {
            if (hierarchy[i].Id == nodeId)
            {
                var removed = hierarchy[i];
                hierarchy.RemoveAt(i);
                return removed;
            }
            if (hierarchy[i] is FolderNode folder)
            {
                var found = RemoveNode(folder.Children, nodeId);
                if (found != null) return found;
            }
        }
        return null;
    }

    public static TreeNode? FindNode(List<TreeNode> hierarchy, string nodeId)
    {
        foreach (var node in hierarchy)
        {
            if (node.Id == nodeId) return node;
            if (node is FolderNode folder)
            {
                var found = FindNode(folder.Children, nodeId);
                if (found != null) return found;
            }
        }
        return null;
    }

    public static string? FindParentId(List<TreeNode> hierarchy, string nodeId)
    {
        foreach (var node in hierarchy)
        {
            if (node is FolderNode folder)
            {
                if (folder.Children.Any(c => c.Id == nodeId)) return folder.Id;
                var found = FindParentId(folder.Children, nodeId);
                if (found != null) return found;
            }
        }
        return null;
    }

    public static bool MoveNode(List<TreeNode> hierarchy, string nodeId, string? newParentId)
    {
        var node = RemoveNode(hierarchy, nodeId);
        if (node == null) return false;

        if (newParentId == null)
            hierarchy.Add(node);
        else
            AddChildToFolder(hierarchy, newParentId, node);

        return true;
    }

    public static bool ReorderNode(List<TreeNode> hierarchy, string nodeId, int newIndex)
    {
        for (int i = 0; i < hierarchy.Count; i++)
        {
            if (hierarchy[i].Id == nodeId)
            {
                var node = hierarchy[i];
                hierarchy.RemoveAt(i);
                var clampedIndex = Math.Clamp(newIndex, 0, hierarchy.Count);
                hierarchy.Insert(clampedIndex, node);
                return true;
            }
            if (hierarchy[i] is FolderNode folder && ReorderNode(folder.Children, nodeId, newIndex))
                return true;
        }
        return false;
    }

    public static List<Document> CollectManuscriptDocs(List<TreeNode> hierarchy, Dictionary<string, Document> documents)
    {
        var manuscript = FindNode(hierarchy, "manuscript") as FolderNode
                         ?? hierarchy.OfType<FolderNode>().FirstOrDefault(f => f.Name.Equals("Manuscript", StringComparison.OrdinalIgnoreCase));

        var result = new List<Document>();
        if (manuscript != null)
            CollectDocs(manuscript.Children, documents, result);
        return result;
    }

    private static void CollectDocs(List<TreeNode> nodes, Dictionary<string, Document> documents, List<Document> result)
    {
        foreach (var node in nodes)
        {
            if (node is DocumentNode dn && documents.TryGetValue(dn.Id, out var doc) && doc.IncludeInCompile)
                result.Add(doc);
            else if (node is FolderNode folder)
                CollectDocs(folder.Children, documents, result);
        }
    }

    public static FolderNode? FindFolder(List<TreeNode> hierarchy, string name)
    {
        foreach (var node in hierarchy)
        {
            if (node is FolderNode f)
            {
                if (f.Name.Equals(name, StringComparison.OrdinalIgnoreCase)) return f;
                var nested = FindFolder(f.Children, name);
                if (nested != null) return nested;
            }
        }
        return null;
    }

    public static string? GetDocumentPath(List<TreeNode> hierarchy, string docId)
    {
        var node = FindNode(hierarchy, docId) as DocumentNode;
        return node?.Path;
    }
}
