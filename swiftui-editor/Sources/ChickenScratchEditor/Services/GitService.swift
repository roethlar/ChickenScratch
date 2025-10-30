import Foundation

struct GitOperationResult {
    let success: Bool
    let message: String
}

struct GitStatus {
    let repositoryStatus: GitRepositoryStatus
    let branch: String?
    let upstream: String?
    let ahead: Int
    let behind: Int
    let entries: [GitStatusEntry]
}

struct GitStatusEntry: Identifiable {
    enum ChangeType: String {
        case added
        case modified
        case deleted
        case renamed
        case copied
        case untracked
        case ignored
        case unknown

        var displayName: String {
            switch self {
            case .added: return "Added"
            case .modified: return "Modified"
            case .deleted: return "Deleted"
            case .renamed: return "Renamed"
            case .copied: return "Copied"
            case .untracked: return "New"
            case .ignored: return "Ignored"
            case .unknown: return "Changed"
            }
        }
    }

    let id = UUID()
    let path: String
    let change: ChangeType
}

enum GitRepositoryStatus {
    case missing
    case available
}

final class GitService {
    func repositoryStatus(at url: URL) -> GitRepositoryStatus {
        let gitFolder = url.appendingPathComponent(".git")
        return FileManager.default.fileExists(atPath: gitFolder.path) ? .available : .missing
    }

    func initializeRepository(at url: URL) -> GitOperationResult {
        let result = Shell.run(["git", "init"], workingDirectory: url)
        guard result.succeeded else {
            return GitOperationResult(success: false, message: result.stderr)
        }
        return GitOperationResult(success: true, message: "Initialized revisions workspace.")
    }

    func status(at url: URL) -> GitStatus {
        guard repositoryStatus(at: url) == .available else {
            return GitStatus(repositoryStatus: .missing, branch: nil, upstream: nil, ahead: 0, behind: 0, entries: [])
        }

        let porcelain = Shell.run(["git", "status", "--porcelain", "-b"], workingDirectory: url)
        let lines = porcelain.stdout.split(separator: "\n")
        var branch: String?
        var upstream: String?
        var ahead = 0
        var behind = 0
        var entries: [GitStatusEntry] = []

        for (index, line) in lines.enumerated() {
            if index == 0 {
                let info = parseBranchLine(String(line))
                branch = info.branch
                upstream = info.upstream
                ahead = info.ahead
                behind = info.behind
                continue
            }
            guard line.count >= 4 else { continue }
            let statusCode = String(line.prefix(2))
            let path = line.dropFirst(3)
            let change = mapChangeType(statusCode)
            entries.append(GitStatusEntry(path: String(path), change: change))
        }

        return GitStatus(
            repositoryStatus: .available,
            branch: branch,
            upstream: upstream,
            ahead: ahead,
            behind: behind,
            entries: entries
        )
    }

    func stageAll(at url: URL) -> GitOperationResult {
        let result = Shell.run(["git", "add", "--all"], workingDirectory: url)
        return GitOperationResult(success: result.succeeded, message: result.succeeded ? "Staged changes." : result.stderr)
    }

    func commit(at url: URL, message: String) -> GitOperationResult {
        let stageResult = stageAll(at: url)
        guard stageResult.success else { return stageResult }

        let result = Shell.run(["git", "commit", "-m", message], workingDirectory: url)
        if result.succeeded {
            return GitOperationResult(success: true, message: "Saved revision.")
        }

        if result.stderr.contains("nothing to commit") {
            return GitOperationResult(success: false, message: "No changes to commit.")
        }

        return GitOperationResult(success: false, message: result.stderr)
    }

    func createBranch(at url: URL, name: String) -> GitOperationResult {
        let result = Shell.run(["git", "checkout", "-b", name], workingDirectory: url)
        return GitOperationResult(success: result.succeeded, message: result.succeeded ? "Created revision \(name)." : result.stderr)
    }

    func checkoutBranch(at url: URL, name: String) -> GitOperationResult {
        let result = Shell.run(["git", "checkout", name], workingDirectory: url)
        return GitOperationResult(success: result.succeeded, message: result.succeeded ? "Switched to revision \(name)." : result.stderr)
    }

    func listBranches(at url: URL) -> [String] {
        let result = Shell.run(["git", "branch", "--list"], workingDirectory: url)
        guard result.succeeded else {
            return []
        }
        return result.stdout
            .split(separator: "\n")
            .map { $0.replacingOccurrences(of: "*", with: "").trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
    }

    func push(at url: URL) -> GitOperationResult {
        let result = Shell.run(["git", "push"], workingDirectory: url)
        return GitOperationResult(success: result.succeeded, message: result.succeeded ? "Pushed revisions." : result.stderr)
    }

    func pull(at url: URL) -> GitOperationResult {
        let result = Shell.run(["git", "pull"], workingDirectory: url)
        return GitOperationResult(success: result.succeeded, message: result.succeeded ? "Pulled latest revisions." : result.stderr)
    }

    private func mapChangeType(_ code: String) -> GitStatusEntry.ChangeType {
        switch code {
        case "A ", "AM", "AA": return .added
        case " M", "MM", "RM", "CM": return .modified
        case " D", "DD": return .deleted
        case "R ", "RM": return .renamed
        case "C ", "CM": return .copied
        case "??": return .untracked
        case "!!": return .ignored
        default: return .unknown
        }
    }

    private func parseBranchLine(_ line: String) -> (branch: String?, upstream: String?, ahead: Int, behind: Int) {
        // Examples:
        // ## main
        // ## main...origin/main [ahead 1]
        // ## main...origin/main [ahead 1, behind 2]
        guard line.hasPrefix("##") else { return (nil, nil, 0, 0) }
        let components = line.replacingOccurrences(of: "## ", with: "").split(separator: " ")
        guard let primary = components.first else {
            return (nil, nil, 0, 0)
        }

        let branchParts = primary.split(separator: "...")
        let branch = branchParts.first.map(String.init)
        let upstream = branchParts.count > 1 ? String(branchParts[1]) : nil

        var ahead = 0
        var behind = 0
        if components.count > 1 {
            let statusPart = components[1...].joined(separator: " ")
            if let aheadRange = statusPart.range(of: "ahead ") {
                let substring = statusPart[aheadRange.upperBound...]
                if let number = substring.split(separator: ",").first.flatMap({ Int($0.replacingOccurrences(of: "]", with: "")) }) {
                    ahead = number
                }
            }
            if let behindRange = statusPart.range(of: "behind ") {
                let substring = statusPart[behindRange.upperBound...]
                if let number = substring.split(separator: ",").first.flatMap({ Int($0.replacingOccurrences(of: "]", with: "")) }) {
                    behind = number
                }
            }
        }

        return (branch, upstream, ahead, behind)
    }
}
