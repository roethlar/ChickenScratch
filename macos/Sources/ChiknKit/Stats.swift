import Foundation

/// One day's manuscript word-count snapshot. Mirrors the Tauri shape stored
/// in `<project>/settings/writing-history.json`. `startWords` is captured the
/// first time `recordDailyWords` runs in a given day so "today's writing" can
/// be computed as `current - startWords` rather than `current - yesterday`.
public struct DayEntry: Sendable, Codable, Equatable {
    public let date: String         // YYYY-MM-DD, UTC
    public var words: Int
    public var startWords: Int?

    enum CodingKeys: String, CodingKey {
        case date, words
        case startWords = "start_words"
    }

    public init(date: String, words: Int, startWords: Int?) {
        self.date = date
        self.words = words
        self.startWords = startWords
    }
}

public struct WritingHistory: Sendable, Codable, Equatable {
    public var entries: [DayEntry]
    public init(entries: [DayEntry] = []) { self.entries = entries }
}

public struct ProjectStats: Sendable, Equatable {
    public let totalWords: Int
    public let manuscriptWords: Int
    public let totalDocs: Int
    public let docs: [DocStats]

    public init(totalWords: Int, manuscriptWords: Int, totalDocs: Int, docs: [DocStats]) {
        self.totalWords = totalWords
        self.manuscriptWords = manuscriptWords
        self.totalDocs = totalDocs
        self.docs = docs
    }
}

public struct DocStats: Sendable, Equatable, Identifiable {
    public let id: String
    public let name: String
    public let words: Int
    public let includeInCompile: Bool
    public init(id: String, name: String, words: Int, includeInCompile: Bool) {
        self.id = id
        self.name = name
        self.words = words
        self.includeInCompile = includeInCompile
    }
}

public struct SessionProgress: Sendable, Equatable {
    public let todayWords: Int
    public let wordsPerSession: Int?
    public let totalTarget: Int?
    public let deadline: String?
    public let daysRemaining: Int?
    public let currentTotal: Int
    /// `(totalTarget - currentTotal) / daysRemaining`, rounded up. nil when
    /// no target / deadline configured, or deadline already passed.
    public let neededPerDay: Int?

    public init(
        todayWords: Int,
        wordsPerSession: Int?,
        totalTarget: Int?,
        deadline: String?,
        daysRemaining: Int?,
        currentTotal: Int,
        neededPerDay: Int?
    ) {
        self.todayWords = todayWords
        self.wordsPerSession = wordsPerSession
        self.totalTarget = totalTarget
        self.deadline = deadline
        self.daysRemaining = daysRemaining
        self.currentTotal = currentTotal
        self.neededPerDay = neededPerDay
    }
}

public enum Stats {
    /// Word counter that strips inline HTML before counting and skips pure
    /// markdown punctuation tokens (#, *, -, …). Mirrors `count_words_md`
    /// in the Tauri io commands.
    public static func wordCount(markdown: String) -> Int {
        var text = String()
        text.reserveCapacity(markdown.count)
        var inTag = false
        for ch in markdown {
            if ch == "<" {
                inTag = true
            } else if ch == ">" {
                inTag = false
                text.append(" ")
            } else if !inTag {
                text.append(ch)
            }
        }
        let punctuation: Set<Character> = ["#", "*", "-", "_", "`", ">", "=", "|"]
        return text.split { $0.isWhitespace || $0.isNewline }
            .filter { token in
                !token.allSatisfy { punctuation.contains($0) }
            }
            .count
    }

    /// Per-document and aggregate stats. Order is "biggest first" so the UI
    /// can render a sensible bar list.
    public static func projectStats(_ project: Project) -> ProjectStats {
        var docs: [DocStats] = []
        var total = 0
        var manuscript = 0
        for doc in project.documents.values where doc.relativePath.hasSuffix(".md") {
            let words = wordCount(markdown: doc.content)
            total += words
            if doc.relativePath.hasPrefix("manuscript/") {
                manuscript += words
            }
            docs.append(DocStats(id: doc.id, name: doc.name, words: words, includeInCompile: doc.meta.includeInCompile))
        }
        docs.sort { $0.words > $1.words }
        return ProjectStats(totalWords: total, manuscriptWords: manuscript, totalDocs: docs.count, docs: docs)
    }

    public static func writingHistoryURL(in projectURL: URL) -> URL {
        projectURL.appendingPathComponent("settings").appendingPathComponent("writing-history.json")
    }

    public static func loadWritingHistory(in projectURL: URL) -> WritingHistory {
        let url = writingHistoryURL(in: projectURL)
        guard FileManager.default.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url),
              let history = try? JSONDecoder().decode(WritingHistory.self, from: data)
        else {
            return WritingHistory()
        }
        return history
    }

    /// Record the project's current manuscript word count for today. The
    /// first call of the day captures `startWords`; subsequent calls update
    /// `words`. Older entries that pre-date the field deserialize with
    /// `startWords == nil`, which the badge treats as "0 today" gracefully.
    @discardableResult
    public static func recordDailyWords(_ words: Int, in projectURL: URL) throws -> WritingHistory {
        let dir = projectURL.appendingPathComponent("settings")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let url = writingHistoryURL(in: projectURL)

        var history = loadWritingHistory(in: projectURL)
        let today = todayString()

        if let idx = history.entries.firstIndex(where: { $0.date == today }) {
            if history.entries[idx].startWords == nil {
                history.entries[idx].startWords = history.entries[idx].words
            }
            history.entries[idx].words = words
        } else {
            history.entries.append(DayEntry(date: today, words: words, startWords: words))
        }
        if history.entries.count > 90 {
            history.entries = Array(history.entries.suffix(90))
        }
        let data = try JSONEncoder().encode(history)
        try data.write(to: url, options: .atomic)
        return history
    }

    /// Compute today's session progress against the project's session target.
    public static func sessionProgress(_ project: Project) -> SessionProgress {
        let target = project.metadata.sessionTarget ?? SessionTarget()
        let stats = projectStats(project)
        let currentTotal = stats.manuscriptWords

        let history = loadWritingHistory(in: project.path)
        let today = todayString()
        let todayWords: Int
        if let entry = history.entries.first(where: { $0.date == today }),
           let start = entry.startWords {
            todayWords = currentTotal - start
        } else {
            todayWords = 0
        }

        let daysRemaining: Int?
        if let deadlineString = target.deadline,
           let deadline = parseDateOnly(deadlineString) {
            let now = Calendar(identifier: .gregorian).startOfDay(for: Date())
            let deadlineStart = Calendar(identifier: .gregorian).startOfDay(for: deadline)
            let comps = Calendar(identifier: .gregorian).dateComponents([.day], from: now, to: deadlineStart)
            daysRemaining = comps.day
        } else {
            daysRemaining = nil
        }

        let neededPerDay: Int?
        if let total = target.totalTarget, let days = daysRemaining, days > 0, total > currentTotal {
            let remaining = total - currentTotal
            // Round up.
            neededPerDay = (remaining + days - 1) / days
        } else {
            neededPerDay = nil
        }

        return SessionProgress(
            todayWords: max(0, todayWords),
            wordsPerSession: target.wordsPerSession,
            totalTarget: target.totalTarget,
            deadline: target.deadline,
            daysRemaining: daysRemaining,
            currentTotal: currentTotal,
            neededPerDay: neededPerDay
        )
    }

    // MARK: - Date helpers

    private static func todayString() -> String {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: Date())
    }

    private static func parseDateOnly(_ s: String) -> Date? {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.date(from: s)
    }
}
