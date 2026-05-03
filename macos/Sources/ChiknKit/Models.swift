import Foundation

public struct Project: Sendable, Identifiable {
    public let id: String
    public var name: String
    public var path: URL
    public var created: Date
    public var modified: Date
    public var hierarchy: [TreeNode]
    public var metadata: ProjectMetadata
    public var documents: [String: Document]
    /// Plot threads (novelist convention; persisted in `threads.yaml`).
    /// Empty for projects that don't use them.
    public var threads: [Thread]

    public init(
        id: String,
        name: String,
        path: URL,
        created: Date,
        modified: Date,
        hierarchy: [TreeNode],
        metadata: ProjectMetadata,
        documents: [String: Document],
        threads: [Thread] = []
    ) {
        self.id = id
        self.name = name
        self.path = path
        self.created = created
        self.modified = modified
        self.hierarchy = hierarchy
        self.metadata = metadata
        self.documents = documents
        self.threads = threads
    }
}

public struct ProjectMetadata: Sendable {
    public var title: String?
    public var author: String?
    public var projectType: String?
    public var genre: String?
    public var theme: String?
    public var summary: String?
    public var sessionTarget: SessionTarget?

    public init(
        title: String? = nil,
        author: String? = nil,
        projectType: String? = nil,
        genre: String? = nil,
        theme: String? = nil,
        summary: String? = nil,
        sessionTarget: SessionTarget? = nil
    ) {
        self.title = title
        self.author = author
        self.projectType = projectType
        self.genre = genre
        self.theme = theme
        self.summary = summary
        self.sessionTarget = sessionTarget
    }
}

/// Writer session targets — words/session goal, optional deadline, total target.
/// All optional; all-nil means the feature is off for this project.
public struct SessionTarget: Sendable, Equatable {
    public var wordsPerSession: Int?
    public var deadline: String?       // YYYY-MM-DD
    public var totalTarget: Int?

    public init(wordsPerSession: Int? = nil, deadline: String? = nil, totalTarget: Int? = nil) {
        self.wordsPerSession = wordsPerSession
        self.deadline = deadline
        self.totalTarget = totalTarget
    }

    public var isEmpty: Bool {
        wordsPerSession == nil && deadline == nil && totalTarget == nil
    }
}

/// A plot thread — novelist UI convention persisted at the project root in
/// `threads.yaml`. The format itself stays genre-agnostic; this lives in the
/// project model so any frontend that wants threads has a typed view of them.
public struct Thread: Sendable, Identifiable, Equatable {
    public let id: String       // slug-style; referenced from a document's fields["threads"]
    public var name: String
    public var color: String?
    public var description: String?

    public init(id: String, name: String, color: String? = nil, description: String? = nil) {
        self.id = id
        self.name = name
        self.color = color
        self.description = description
    }
}

public enum NodeKind: String, Sendable {
    case document
    case folder
}

public struct TreeNode: Sendable, Identifiable, Hashable {
    public let id: String
    public var name: String
    public var kind: NodeKind
    public var children: [TreeNode]

    public init(id: String, name: String, kind: NodeKind, children: [TreeNode] = []) {
        self.id = id
        self.name = name
        self.kind = kind
        self.children = children
    }

    public static func == (lhs: TreeNode, rhs: TreeNode) -> Bool { lhs.id == rhs.id }
    public func hash(into hasher: inout Hasher) { hasher.combine(id) }
}

public struct Document: Sendable, Identifiable {
    public let id: String
    public var name: String
    public var relativePath: String
    public var content: String
    public var meta: DocumentMeta

    public init(id: String, name: String, relativePath: String, content: String, meta: DocumentMeta) {
        self.id = id
        self.name = name
        self.relativePath = relativePath
        self.content = content
        self.meta = meta
    }
}

public struct DocumentMeta: Sendable {
    public var synopsis: String?
    public var label: String?
    public var status: String?
    public var keywords: [String]
    public var includeInCompile: Bool
    public var wordCountTarget: Int?
    public var compileOrder: Int?
    /// Generic UI extensibility — the format's sole point of extension. Keys
    /// follow per-domain convention docs (see UI_CONVENTIONS_NOVELIST.md for
    /// `pov_character`, `location`, `story_time`, `duration_minutes`,
    /// `threads`, `characters_in_scene`, `entity_kind`).
    ///
    /// "Tolerant readers, preserving writers": any keys present on disk that
    /// don't match the typed columns above land here on read, and they're
    /// written back unchanged on save.
    public var fields: [String: YAMLValue]

    public init(
        synopsis: String? = nil,
        label: String? = nil,
        status: String? = nil,
        keywords: [String] = [],
        includeInCompile: Bool = true,
        wordCountTarget: Int? = nil,
        compileOrder: Int? = nil,
        fields: [String: YAMLValue] = [:]
    ) {
        self.synopsis = synopsis
        self.label = label
        self.status = status
        self.keywords = keywords
        self.includeInCompile = includeInCompile
        self.wordCountTarget = wordCountTarget
        self.compileOrder = compileOrder
        self.fields = fields
    }
}

/// Sendable, typed mirror of a YAML scalar/sequence/mapping. Used so
/// `DocumentMeta.fields` can carry arbitrary user-defined entries while
/// keeping the rest of the model `Sendable`.
public enum YAMLValue: Sendable, Equatable {
    case null
    case bool(Bool)
    case int(Int)
    case double(Double)
    case string(String)
    indirect case array([YAMLValue])
    indirect case dict([String: YAMLValue])

    /// Best-effort conversion from the `Any` shape Yams produces. Order of
    /// checks matters: Bool must be tested before Int because Foundation's
    /// `NSNumber` bridge can answer yes to both.
    public init?(any: Any) {
        if any is NSNull {
            self = .null
            return
        }
        // Yams (libyaml) eagerly parses ISO-shaped scalars into Date per the
        // YAML 1.1 schema. Round-trip those back to a String so the format's
        // `fields` map stays a free-text key-value store from the UI's POV —
        // a `story_time: 2026-04-23` written by one frontend should read back
        // as the same string from another.
        if let d = any as? Date {
            self = .string(YAMLValue.formatDateForYAML(d))
            return
        }
        // Detect NSNumber holding a Bool before letting it match Int/Double.
        if let num = any as? NSNumber {
            let typeStr = String(cString: num.objCType)
            if typeStr == "c" || typeStr == "B" {
                self = .bool(num.boolValue)
                return
            }
        }
        switch any {
        case let b as Bool: self = .bool(b)
        case let i as Int: self = .int(i)
        case let d as Double: self = .double(d)
        case let s as String: self = .string(s)
        case let a as [Any]:
            self = .array(a.compactMap(YAMLValue.init(any:)))
        case let m as [String: Any]:
            var out: [String: YAMLValue] = [:]
            for (k, v) in m {
                if let yv = YAMLValue(any: v) { out[k] = yv }
            }
            self = .dict(out)
        default:
            return nil
        }
    }

    private static func formatDateForYAML(_ date: Date) -> String {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "UTC") ?? .gmt
        let parts = calendar.dateComponents(
            [.hour, .minute, .second, .nanosecond],
            from: date
        )
        let isDateOnly = (parts.hour ?? 0) == 0
            && (parts.minute ?? 0) == 0
            && (parts.second ?? 0) == 0
            && (parts.nanosecond ?? 0) == 0
        let formatter = ISO8601DateFormatter()
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.formatOptions = isDateOnly ? [.withFullDate] : [.withInternetDateTime]
        return formatter.string(from: date)
    }

    /// Convert back to Foundation-compatible `Any` for Yams to serialize.
    public func toAny() -> Any {
        switch self {
        case .null: return NSNull()
        case .bool(let b): return b
        case .int(let i): return i
        case .double(let d): return d
        case .string(let s): return s
        case .array(let arr): return arr.map { $0.toAny() }
        case .dict(let m):
            var out: [String: Any] = [:]
            for (k, v) in m { out[k] = v.toAny() }
            return out
        }
    }

    public var asString: String? {
        if case .string(let s) = self { return s }
        return nil
    }

    public var asInt: Int? {
        switch self {
        case .int(let i): return i
        case .double(let d): return Int(d)
        default: return nil
        }
    }

    public var asBool: Bool? {
        if case .bool(let b) = self { return b }
        return nil
    }

    public var asStringArray: [String]? {
        guard case .array(let arr) = self else { return nil }
        return arr.compactMap { $0.asString }
    }

    /// True for empty string / empty list / empty mapping / null. Used by the
    /// writer to remove a key from `fields` rather than persist a stored empty.
    public var isStorageEmpty: Bool {
        switch self {
        case .null: return true
        case .string(let s): return s.isEmpty
        case .array(let arr): return arr.isEmpty
        case .dict(let m): return m.isEmpty
        case .bool, .int, .double: return false
        }
    }
}

public enum ChiknError: Error, LocalizedError {
    case notAChiknFolder(URL)
    case invalidProjectYaml(String)
    case documentMissing(String)
    case io(Error)

    public var errorDescription: String? {
        switch self {
        case .notAChiknFolder(let url): "Not a .chikn project: \(url.path)"
        case .invalidProjectYaml(let msg): "project.yaml is invalid: \(msg)"
        case .documentMissing(let path): "Document missing on disk: \(path)"
        case .io(let err): err.localizedDescription
        }
    }
}
