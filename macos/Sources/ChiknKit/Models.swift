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

    public init(
        id: String,
        name: String,
        path: URL,
        created: Date,
        modified: Date,
        hierarchy: [TreeNode],
        metadata: ProjectMetadata,
        documents: [String: Document]
    ) {
        self.id = id
        self.name = name
        self.path = path
        self.created = created
        self.modified = modified
        self.hierarchy = hierarchy
        self.metadata = metadata
        self.documents = documents
    }
}

public struct ProjectMetadata: Sendable {
    public var title: String?
    public var author: String?
    public var projectType: String?
    public var genre: String?
    public var theme: String?
    public var summary: String?

    public init(
        title: String? = nil,
        author: String? = nil,
        projectType: String? = nil,
        genre: String? = nil,
        theme: String? = nil,
        summary: String? = nil
    ) {
        self.title = title
        self.author = author
        self.projectType = projectType
        self.genre = genre
        self.theme = theme
        self.summary = summary
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

    public init(
        synopsis: String? = nil,
        label: String? = nil,
        status: String? = nil,
        keywords: [String] = [],
        includeInCompile: Bool = true,
        wordCountTarget: Int? = nil,
        compileOrder: Int? = nil
    ) {
        self.synopsis = synopsis
        self.label = label
        self.status = status
        self.keywords = keywords
        self.includeInCompile = includeInCompile
        self.wordCountTarget = wordCountTarget
        self.compileOrder = compileOrder
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
