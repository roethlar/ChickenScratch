using ChickenScratch.Core.Git;
using LibGit2Sharp;

var repoPath = Path.Combine(Path.GetTempPath(), "chickenscratch-git-restore-" + Guid.NewGuid().ToString("N"));
Directory.CreateDirectory(repoPath);

try
{
    GitService.Init(repoPath);

    var manuscriptPath = Path.Combine(repoPath, "manuscript.txt");
    var addedLaterPath = Path.Combine(repoPath, "added-later.txt");

    File.WriteAllText(manuscriptPath, "first revision");
    var firstRevision = GitService.SaveRevision(repoPath, "first");

    File.WriteAllText(manuscriptPath, "second revision");
    File.WriteAllText(addedLaterPath, "added in second revision");
    var previousHead = GitService.SaveRevision(repoPath, "second");

    var restoredRevision = GitService.RestoreRevision(repoPath, firstRevision.Id);

    using var repo = new Repository(repoPath);
    var firstCommit = repo.Lookup<Commit>(firstRevision.Id)
        ?? throw new InvalidOperationException("first commit was not found");
    var previousHeadCommit = repo.Lookup<Commit>(previousHead.Id)
        ?? throw new InvalidOperationException("previous HEAD commit was not found");
    var restoredCommit = repo.Lookup<Commit>(restoredRevision.Id)
        ?? throw new InvalidOperationException("restored commit was not found");

    AssertEqual(restoredCommit.Sha, repo.Head.Tip.Sha, "restore should leave HEAD at the new commit");
    AssertEqual(1, restoredCommit.Parents.Count(), "restore commit should have exactly one parent");
    AssertEqual(previousHeadCommit.Sha, restoredCommit.Parents.Single().Sha, "restore commit should preserve prior HEAD as parent");
    AssertEqual(firstCommit.Tree.Id, restoredCommit.Tree.Id, "restore commit tree should match restored target tree");
    AssertEqual("first revision", File.ReadAllText(manuscriptPath), "worktree content should match restored target");

    if (File.Exists(addedLaterPath))
        throw new InvalidOperationException("restore should remove files that do not exist in the restored target tree");

    Console.WriteLine("GitServiceRestoreHarness: passed");
    return 0;
}
finally
{
    try
    {
        Directory.Delete(repoPath, recursive: true);
    }
    catch
    {
        // Best-effort cleanup for diagnostic runs.
    }
}

static void AssertEqual<T>(T expected, T actual, string message)
{
    if (!EqualityComparer<T>.Default.Equals(expected, actual))
        throw new InvalidOperationException($"{message}. Expected: {expected}; actual: {actual}");
}
