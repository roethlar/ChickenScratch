// Raw YAML shapes that map 1:1 with the on-disk format.
//
// These mirror `crates/core/src/core/project/reader.rs::DocumentMetadata` and
// the sibling `Project*` shapes. Any field the format owns must appear here so
// that closed-POCO YAML serialization round-trips it instead of dropping it.

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
    public SessionTargetYaml? SessionTarget { get; set; }
}

internal class SessionTargetYaml
{
    public uint? WordsPerSession { get; set; }
    public string? Deadline { get; set; }
    public uint? TotalTarget { get; set; }
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
    // ── Identity (required for cross-frontend round-trip) ─────────────
    // The Rust reader keys `project.documents` from `meta.id`. Without these
    // fields the Rust/Tauri/Swift readers can synthesize fresh ids that don't
    // match the hierarchy node, leaving the binder pointing at "missing" docs.
    public string? Id { get; set; }
    public string? Name { get; set; }
    public string? ParentId { get; set; }
    public DateTime Created { get; set; }
    public DateTime Modified { get; set; }

    // ── Editorial metadata ────────────────────────────────────────────
    public string? Synopsis { get; set; }
    public string? Label { get; set; }
    public string? Status { get; set; }
    public List<string>? Keywords { get; set; }
    public List<string>? Links { get; set; }

    /// <summary>
    /// Compile inclusion flag. Canonical wire form is `"Yes"`/`"No"` strings —
    /// matches the Rust writer (`crates/core/src/core/project/writer.rs`).
    /// On read, the Rust reader also accepts a YAML bool for back-compat with
    /// older Windows-written `.meta` files; the Windows reader does the same
    /// via <see cref="ProjectReader.DecodeIncludeInCompile"/>.
    /// </summary>
    public string? IncludeInCompile { get; set; }

    public uint WordCountTarget { get; set; } = 0;
    public int CompileOrder { get; set; } = 0;

    // ── Scrivener round-trip ──────────────────────────────────────────
    public string? SectionType { get; set; }
    public string? ScrivenerUuid { get; set; }

    // ── Inline comments ───────────────────────────────────────────────
    public List<CommentYaml>? Comments { get; set; }

    /// <summary>
    /// Generic UI extensibility — see CHIKN_FORMAT_SPEC.md v1.2.
    /// The format does not interpret entries here; readers preserve them on
    /// round-trip. Per-domain key conventions live in separate UI docs.
    /// </summary>
    public Dictionary<string, object?>? Fields { get; set; }
}

internal class CommentYaml
{
    public string Id { get; set; } = string.Empty;
    public string Body { get; set; } = string.Empty;
    public bool Resolved { get; set; }
    public string Created { get; set; } = string.Empty;
    public string Modified { get; set; } = string.Empty;
}

/// <summary>Root of `threads.yaml`. Optional sidecar at the project root.</summary>
internal class ThreadsYamlRoot
{
    public List<ThreadYaml> Threads { get; set; } = [];
}

internal class ThreadYaml
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public string? Color { get; set; }
    public string? Description { get; set; }
}
