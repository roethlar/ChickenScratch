import Foundation

/// Lightweight wrapper around the system `git` binary. The other platform
/// implementations use libgit2 (Rust git2-rs, C# LibGit2Sharp); on macOS we
/// shell out because SwiftPM doesn't have a zero-friction libgit2 wrapper
/// that ships with the SDK. Every recent macOS has `git` at /usr/bin/git via
/// Command Line Tools, which routes through xcrun.
public enum Git {
    public struct GitError: Error, LocalizedError {
        public let message: String
        public var errorDescription: String? { message }
    }

    public struct RevisionEntry: Identifiable, Hashable, Sendable {
        public let id: String        // full hash
        public let shortId: String   // first 8 chars
        public let message: String
        public let date: Date
    }

    /// Initialize a git repo if one doesn't already exist. Writes .gitignore
    /// on first init to match the format spec.
    public static func initRepoIfNeeded(at projectURL: URL) throws {
        let gitDir = projectURL.appendingPathComponent(".git")
        if FileManager.default.fileExists(atPath: gitDir.path) { return }

        try run(["init", "-q"], in: projectURL)

        let gitignore = projectURL.appendingPathComponent(".gitignore")
        if !FileManager.default.fileExists(atPath: gitignore.path) {
            let content = "revs/\n.DS_Store\nThumbs.db\n*.tmp\n*.swp\n*~\n"
            try content.write(to: gitignore, atomically: true, encoding: .utf8)
        }
    }

    /// True if the working tree has staged or unstaged changes.
    public static func hasChanges(in projectURL: URL) throws -> Bool {
        let output = try runCapturing(["status", "--porcelain"], in: projectURL)
        return !output.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    /// Stage everything and commit with the given message. No-op if the tree
    /// is clean. Returns the short commit hash on success.
    @discardableResult
    public static func saveRevision(
        message: String,
        in projectURL: URL,
        authorName: String = "ChickenScratch",
        authorEmail: String = "writer@chickenscratch.local"
    ) throws -> String? {
        try initRepoIfNeeded(at: projectURL)
        guard try hasChanges(in: projectURL) else { return nil }

        try run(["add", "-A"], in: projectURL)

        var env = ProcessInfo.processInfo.environment
        env["GIT_AUTHOR_NAME"] = authorName
        env["GIT_AUTHOR_EMAIL"] = authorEmail
        env["GIT_COMMITTER_NAME"] = authorName
        env["GIT_COMMITTER_EMAIL"] = authorEmail

        try run(["commit", "-q", "-m", message], in: projectURL, env: env)

        let short = try runCapturing(["rev-parse", "--short", "HEAD"], in: projectURL)
        return short.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// Return up to 60 recent commits in reverse chronological order.
    public static func listRevisions(in projectURL: URL) throws -> [RevisionEntry] {
        // git log --format="%H|%s|%aI" -60
        let raw = try runCapturing(["log", "--format=%H|%s|%aI", "-60"], in: projectURL)
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return raw.split(separator: "\n", omittingEmptySubsequences: true).compactMap { line in
            let parts = line.split(separator: "|", maxSplits: 2, omittingEmptySubsequences: false)
            guard parts.count == 3 else { return nil }
            let hash = String(parts[0])
            let msg  = String(parts[1])
            let date = formatter.date(from: String(parts[2])) ?? Date.distantPast
            return RevisionEntry(id: hash, shortId: String(hash.prefix(8)), message: msg, date: date)
        }
    }

    /// Restore to `commitHash` by checking out that tree and making a new commit.
    public static func restoreRevision(commitHash: String, in projectURL: URL) throws {
        try run(["checkout", commitHash, "--", "."], in: projectURL)
        var env = ProcessInfo.processInfo.environment
        env["GIT_AUTHOR_NAME"]    = "ChickenScratch"
        env["GIT_AUTHOR_EMAIL"]   = "writer@chickenscratch.local"
        env["GIT_COMMITTER_NAME"] = "ChickenScratch"
        env["GIT_COMMITTER_EMAIL"] = "writer@chickenscratch.local"
        try run(["commit", "-q", "-m", "Restored to \(commitHash.prefix(8))"], in: projectURL, env: env)
    }

    // MARK: - Internals

    private static func run(
        _ args: [String],
        in cwd: URL,
        env: [String: String]? = nil
    ) throws {
        _ = try runCapturing(args, in: cwd, env: env)
    }

    private static func runCapturing(
        _ args: [String],
        in cwd: URL,
        env: [String: String]? = nil
    ) throws -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.currentDirectoryURL = cwd
        process.arguments = args
        if let env { process.environment = env }

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        try process.run()
        process.waitUntilExit()

        let outData = stdout.fileHandleForReading.readDataToEndOfFile()
        let errData = stderr.fileHandleForReading.readDataToEndOfFile()

        if process.terminationStatus != 0 {
            let err = String(data: errData, encoding: .utf8) ?? "unknown error"
            throw GitError(message: "git \(args.first ?? "") failed: \(err.trimmingCharacters(in: .whitespacesAndNewlines))")
        }
        return String(data: outData, encoding: .utf8) ?? ""
    }
}
