import Foundation

struct ChiknProject: Identifiable {
    let id: String
    let name: String
    let created: Date
    var modified: Date
    let url: URL
    var hierarchy: [ChiknTreeNode]
    var documents: [String: ChiknDocument]
}

struct ChiknDocument: Identifiable, Hashable {
    let id: String
    var name: String
    let relativePath: String
    var content: String
    var metadata: DocumentMetadata

    var fileURL: URL {
        metadata.projectURL.appendingPathComponent(relativePath)
    }

    var metaURL: URL {
        let metaPath = relativePath.replacingOccurrences(of: ".md", with: ".meta")
        return metadata.projectURL.appendingPathComponent(metaPath)
    }

    func attributedContent(using transformer: MarkdownTransformer) -> NSAttributedString {
        transformer.attributedString(fromMarkdown: content)
    }
}

struct DocumentMetadata: Hashable {
    let projectURL: URL
    var name: String
    var created: Date
    var modified: Date
    var parentID: String?
    var label: String?
    var status: String?
    var keywords: [String]?
    var synopsis: String?
}

enum ChiknTreeNode: Codable, Identifiable, Hashable {
    case folder(FolderNode)
    case document(DocumentNode)

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(NodeType.self, forKey: .type)
        switch type {
        case .folder:
            let node = try FolderNode(from: decoder)
            self = .folder(node)
        case .document:
            let node = try DocumentNode(from: decoder)
            self = .document(node)
        }
    }

    func encode(to encoder: Encoder) throws {
        switch self {
        case .folder(let node):
            try node.encode(to: encoder)
        case .document(let node):
            try node.encode(to: encoder)
        }
    }

    private enum CodingKeys: String, CodingKey {
        case type
    }

    private enum NodeType: String, Codable {
        case folder = "Folder"
        case document = "Document"
    }

    var id: String {
        switch self {
        case .folder(let node):
            return node.id
        case .document(let node):
            return node.id
        }
    }

    var name: String {
        switch self {
        case .folder(let node):
            return node.name
        case .document(let node):
            return node.name
        }
    }

    struct FolderNode: Codable, Identifiable, Hashable {
        let type: String
        let id: String
        var name: String
        var children: [ChiknTreeNode]

        init(id: String, name: String, children: [ChiknTreeNode]) {
            self.type = "Folder"
            self.id = id
            self.name = name
            self.children = children
        }
    }

    struct DocumentNode: Codable, Identifiable, Hashable {
        let type: String
        let id: String
        var name: String
        var path: String

        init(id: String, name: String, path: String) {
            self.type = "Document"
            self.id = id
            self.name = name
            self.path = path
        }
    }
}
