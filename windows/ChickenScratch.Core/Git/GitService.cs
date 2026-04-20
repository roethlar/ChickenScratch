using ChickenScratch.Core.Models;
using LibGit2Sharp;

namespace ChickenScratch.Core.Git;

public static class GitService
{
    private static readonly Signature Author =
        new("ChickenScratch", "writer@chickenscratch.app", DateTimeOffset.UtcNow);

    public static void Init(string path)
    {
        if (!Repository.IsValid(path))
            Repository.Init(path);
    }

    public static Revision SaveRevision(string path, string message)
    {
        using var repo = new Repository(path);
        Commands.Stage(repo, "*");

        var sig = new Signature(Author.Name, Author.Email, DateTimeOffset.UtcNow);
        var commit = repo.Commit(message, sig, sig, new CommitOptions { AllowEmptyCommit = true });
        return ToRevision(commit);
    }

    public static List<Revision> ListRevisions(string path)
    {
        using var repo = new Repository(path);
        return repo.Commits
            .QueryBy(new CommitFilter { SortBy = CommitSortStrategies.Time })
            .Take(200)
            .Select(ToRevision)
            .ToList();
    }

    public static Revision RestoreRevision(string path, string commitId)
    {
        using var repo = new Repository(path);
        var commit = repo.Lookup<Commit>(commitId)
            ?? throw new InvalidOperationException($"Commit {commitId} not found");

        repo.Reset(ResetMode.Hard, commit);
        var sig = new Signature(Author.Name, Author.Email, DateTimeOffset.UtcNow);
        var restored = repo.Commit($"Restored to: {commit.MessageShort}", sig, sig,
            new CommitOptions { AllowEmptyCommit = true });
        return ToRevision(restored);
    }

    public static void CreateDraft(string path, string name)
    {
        using var repo = new Repository(path);
        repo.CreateBranch(name);
        Commands.Checkout(repo, name);
    }

    public static List<DraftVersion> ListDrafts(string path)
    {
        using var repo = new Repository(path);
        var current = repo.Head.FriendlyName;
        return repo.Branches
            .Where(b => !b.IsRemote)
            .Select(b => new DraftVersion { Name = b.FriendlyName, IsActive = b.FriendlyName == current })
            .ToList();
    }

    public static void SwitchDraft(string path, string name)
    {
        using var repo = new Repository(path);
        Commands.Checkout(repo, name);
    }

    public static void MergeDraft(string path, string name)
    {
        using var repo = new Repository(path);
        var branch = repo.Branches[name]
            ?? throw new InvalidOperationException($"Branch {name} not found");
        var sig = new Signature(Author.Name, Author.Email, DateTimeOffset.UtcNow);
        repo.Merge(branch, sig);
    }

    public static void PushBackup(string projectPath, string backupDir)
    {
        Directory.CreateDirectory(backupDir);
        var repoName = Path.GetFileName(projectPath.TrimEnd(Path.DirectorySeparatorChar));
        var barePath = Path.Combine(backupDir, repoName + ".git");

        if (!Repository.IsValid(barePath))
            Repository.Init(barePath, isBare: true);

        using var repo = new Repository(projectPath);
        var remote = repo.Network.Remotes["backup"]
            ?? repo.Network.Remotes.Add("backup", barePath);
        repo.Network.Push(repo.Head, new PushOptions());
    }

    public static List<FileDiff> RevisionDiff(string path, string commitId)
    {
        using var repo = new Repository(path);
        var commit = repo.Lookup<Commit>(commitId);
        if (commit == null) return [];

        var parent = commit.Parents.FirstOrDefault();
        var oldTree = parent?.Tree;
        var diff = repo.Diff.Compare<TreeChanges>(oldTree, commit.Tree);

        var skipExt = new HashSet<string> { ".meta" };
        var skipFile = new HashSet<string> { "project.yaml" };

        return diff
            .Where(c => !skipExt.Contains(Path.GetExtension(c.Path))
                     && !skipFile.Contains(Path.GetFileName(c.Path))
                     && !c.Path.StartsWith(".git"))
            .Select(c => new FileDiff
            {
                Path = c.Path,
                Status = c.Status switch
                {
                    ChangeKind.Added => "added",
                    ChangeKind.Deleted => "deleted",
                    ChangeKind.Renamed => "renamed",
                    _ => "modified",
                }
            })
            .ToList();
    }

    public static bool HasChanges(string path)
    {
        using var repo = new Repository(path);
        return repo.RetrieveStatus().IsDirty;
    }

    private static Revision ToRevision(Commit c) => new()
    {
        Id = c.Sha,
        ShortId = c.Sha[..8],
        Message = c.MessageShort,
        Timestamp = c.Author.When,
    };
}
