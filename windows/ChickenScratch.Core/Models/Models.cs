using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace ChickenScratch.Core.Models;

// ── Tree ──────────────────────────────────────────────

public abstract class TreeNode
{
    public required string Id { get; set; }
    public required string Name { get; set; }
    public string Type { get; set; } = string.Empty;
}

public class DocumentNode : TreeNode
{
    [SetsRequiredMembers]
    public DocumentNode(string id, string name, string path)
    {
        Id = id; Name = name; Path = path; Type = "document";
    }
    public DocumentNode() => Type = "document";
    public required string Path { get; set; }
}

public class FolderNode : TreeNode
{
    [SetsRequiredMembers]
    public FolderNode(string id, string name)
    {
        Id = id; Name = name; Type = "folder";
    }
    public FolderNode() => Type = "folder";
    public List<TreeNode> Children { get; set; } = [];
}

// ── Document ──────────────────────────────────────────

public class Document
{
    public required string Id { get; set; }
    public required string Name { get; set; }
    public required string Path { get; set; }
    public string Content { get; set; } = string.Empty;
    public string? ParentId { get; set; }
    public DateTime Created { get; set; } = DateTime.UtcNow;
    public DateTime Modified { get; set; } = DateTime.UtcNow;
    public string? Synopsis { get; set; }
    public string? Label { get; set; }
    public string? Status { get; set; }
    public List<string> Keywords { get; set; } = [];
    public List<string> Links { get; set; } = [];
    public bool IncludeInCompile { get; set; } = true;
    public uint WordCountTarget { get; set; } = 0;
    public int CompileOrder { get; set; } = 0;

    /// <summary>Scrivener section type UUID; round-tripped, not interpreted here.</summary>
    public string? SectionType { get; set; }

    /// <summary>Original Scrivener UUID, preserved for round-trip imports/exports.</summary>
    public string? ScrivenerUuid { get; set; }

    /// <summary>Inline comments anchored to spans in the content.</summary>
    public List<Comment> Comments { get; set; } = [];

    /// <summary>
    /// Generic UI extensibility (CHIKN_FORMAT_SPEC.md v1.2). The format owns
    /// nothing inside this map; UIs that recognize a key edit it, UIs that
    /// don't preserve it on round-trip. Required for the format-level
    /// "tolerant readers, preserving writers" guarantee.
    /// </summary>
    public Dictionary<string, object?> Fields { get; set; } = [];
}

/// <summary>
/// Inline comment anchored to a `&lt;span class="comment" data-comment-id="..."&gt;`
/// element inside `Document.Content`. Mirror of `chickenscratch_core::models::Comment`.
/// </summary>
public class Comment
{
    public required string Id { get; set; }
    public string Body { get; set; } = string.Empty;
    public bool Resolved { get; set; }
    public required string Created { get; set; }
    public required string Modified { get; set; }
}

public class DocumentMetaUpdate
{
    public string? Synopsis { get; set; }
    public string? Label { get; set; }
    public string? Status { get; set; }
    public List<string>? Keywords { get; set; }
    public bool? IncludeInCompile { get; set; }
    public uint? WordCountTarget { get; set; }
}

// ── Project ───────────────────────────────────────────

public class ProjectMetadata
{
    public string? Title { get; set; }
    public string? Author { get; set; }
    public string? ProjectType { get; set; }
    public string? Genre { get; set; }
    public string? Theme { get; set; }
    public string? Summary { get; set; }
    public SessionTarget? SessionTarget { get; set; }
}

/// <summary>
/// Writer session targets — words/session goal, optional deadline, total target.
/// All-null means the feature is off for this project; the writer drops the
/// `session_target` key entirely in that case so projects ignoring it write zero diff.
/// </summary>
public class SessionTarget
{
    public uint? WordsPerSession { get; set; }
    public string? Deadline { get; set; }   // ISO date YYYY-MM-DD; free-form on the wire
    public uint? TotalTarget { get; set; }

    public bool IsEmpty =>
        WordsPerSession == null && string.IsNullOrEmpty(Deadline) && TotalTarget == null;
}

/// <summary>
/// Plot thread (novelist convention; persisted in `threads.yaml` at the project root).
/// Stays in the project model so the writer can preserve threads.yaml on round-trip
/// even if the Windows UI doesn't yet surface them.
/// </summary>
public class Thread
{
    public required string Id { get; set; }
    public required string Name { get; set; }
    public string? Color { get; set; }
    public string? Description { get; set; }
}

public class Project
{
    public required string Id { get; set; }
    public required string Name { get; set; }
    public required string Path { get; set; }
    public List<TreeNode> Hierarchy { get; set; } = [];
    public Dictionary<string, Document> Documents { get; set; } = [];
    public DateTime Created { get; set; } = DateTime.UtcNow;
    public DateTime Modified { get; set; } = DateTime.UtcNow;
    public ProjectMetadata Metadata { get; set; } = new();

    /// <summary>Plot threads (novelist convention; round-tripped via `threads.yaml`).</summary>
    public List<Thread> Threads { get; set; } = [];
}

// ── Git ───────────────────────────────────────────────

public class Revision
{
    public required string Id { get; set; }
    public required string ShortId { get; set; }
    public required string Message { get; set; }
    public DateTimeOffset Timestamp { get; set; }
}

public class DraftVersion
{
    public required string Name { get; set; }
    public bool IsActive { get; set; }
}

public class FileDiff
{
    public required string Path { get; set; }
    public required string Status { get; set; } // added / modified / deleted / renamed
}

// ── Compile ───────────────────────────────────────────

public class CompileOptions
{
    public string? Font { get; set; }
    public float? FontSize { get; set; }
    public float? LineSpacing { get; set; }
    public float? MarginInches { get; set; }
    public string SectionSeparator { get; set; } = "* * *";
    public bool IncludeTitlePage { get; set; } = true;
    public bool ManuscriptFormat { get; set; } = false;
}

// ── Settings ──────────────────────────────────────────

public class AppSettings
{
    public GeneralSettings General { get; set; } = new();
    public WritingSettings Writing { get; set; } = new();
    public BackupSettings Backup { get; set; } = new();
    public AiSettings Ai { get; set; } = new();
    public CompileSettings Compile { get; set; } = new();
    public Dictionary<string, string> Shortcuts { get; set; } = DefaultShortcuts();

    public static Dictionary<string, string> DefaultShortcuts() => new()
    {
        ["save"] = "Ctrl+S",
        ["newDocument"] = "Ctrl+N",
        ["search"] = "Ctrl+Shift+P",
        ["commandPalette"] = "Ctrl+K",
        ["focusMode"] = "Ctrl+Shift+F",
        ["toggleBinder"] = "Ctrl+\\",
        ["toggleInspector"] = "Ctrl+Shift+I",
        ["find"] = "Ctrl+F",
        ["findReplace"] = "Ctrl+H",
    };
}

public class GeneralSettings
{
    public string Theme { get; set; } = "dark";
    public int RecentProjectsLimit { get; set; } = 10;
    public string? PandocPath { get; set; }
}

public class WritingSettings
{
    public string FontFamily { get; set; } = "Segoe UI Variable";
    public float FontSize { get; set; } = 16f;
    public string ParagraphStyle { get; set; } = "block";
    public int AutoSaveSeconds { get; set; } = 2;
    public bool SpellCheck { get; set; } = true;
}

public class BackupSettings
{
    public string? BackupDirectory { get; set; }
    public bool AutoBackupOnClose { get; set; } = true;
    public int AutoBackupMinutes { get; set; } = 30;
}

public class AiSettings
{
    public bool Enabled { get; set; } = true;
    public string Provider { get; set; } = "ollama";
    public string? Endpoint { get; set; } = "http://localhost:11434";
    public string? ApiKey { get; set; }
    public string Model { get; set; } = "llama3.2";
}

public class CompileSettings
{
    public string DefaultFormat { get; set; } = "docx";
    public string Font { get; set; } = "Times New Roman";
    public float FontSize { get; set; } = 12f;
    public float LineSpacing { get; set; } = 2f;
    public float MarginInches { get; set; } = 1f;
}

public class RecentProject
{
    public required string Name { get; set; }
    public required string Path { get; set; }
}

// ── Stats / Search ────────────────────────────────────

public class ProjectStats
{
    public int TotalWords { get; set; }
    public int DocumentCount { get; set; }
    public int PageCount => TotalWords / 250;
    public int ReadingMinutes => TotalWords / 200;
}

public class SearchResult
{
    public required string DocId { get; set; }
    public required string DocName { get; set; }
    public required string Snippet { get; set; }
    public int MatchCount { get; set; }
}
