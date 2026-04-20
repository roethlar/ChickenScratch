// Raw YAML shapes that map 1:1 with the on-disk format
namespace ChickenScratch.Core.IO;

internal class ProjectYamlRoot
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public DateTime Created { get; set; }
    public DateTime Modified { get; set; }
    public ProjectMetaYaml? Metadata { get; set; }
    public List<TreeNodeYaml> Hierarchy { get; set; } = [];
}

internal class ProjectMetaYaml
{
    public string? Title { get; set; }
    public string? Author { get; set; }
    public string? ProjectType { get; set; }
    public string? Genre { get; set; }
    public string? Theme { get; set; }
    public string? Summary { get; set; }
}

internal class TreeNodeYaml
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public string Type { get; set; } = string.Empty;
    public string? Path { get; set; }                     // for document nodes
    public List<TreeNodeYaml>? Children { get; set; }    // for folder nodes
}

internal class DocumentMetaYaml
{
    public string? Synopsis { get; set; }
    public string? Label { get; set; }
    public string? Status { get; set; }
    public List<string>? Keywords { get; set; }
    public List<string>? Links { get; set; }
    public bool IncludeInCompile { get; set; } = true;
    public uint WordCountTarget { get; set; } = 0;
    public int CompileOrder { get; set; } = 0;
    public DateTime Created { get; set; }
    public DateTime Modified { get; set; }
}
