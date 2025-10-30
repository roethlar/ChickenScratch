import Foundation
import Yams

final class ChiknProjectWriter {
    private let encoder = YAMLEncoder()

    func saveProject(_ project: ChiknProject) throws {
        let projectFile = ProjectFile(
            id: project.id,
            name: project.name,
            hierarchy: project.hierarchy,
            created: DateParser.encode(project.created),
            modified: DateParser.encode(project.modified)
        )

        let yaml = try encoder.encode(projectFile)
        let targetURL = project.url.appendingPathComponent("project.yaml")
        try yaml.write(to: targetURL, atomically: true, encoding: .utf8)
    }

    func saveDocument(_ document: ChiknDocument) throws {
        try document.content.write(to: document.fileURL, atomically: true, encoding: .utf8)

        let meta = DocumentMetaFile(
            id: document.id,
            name: document.metadata.name,
            created: DateParser.encode(document.metadata.created),
            modified: DateParser.encode(document.metadata.modified),
            parent_id: document.metadata.parentID,
            label: document.metadata.label,
            status: document.metadata.status,
            keywords: document.metadata.keywords,
            synopsis: document.metadata.synopsis
        )

        let metaYAML = try encoder.encode(meta)
        try metaYAML.write(to: document.metaURL, atomically: true, encoding: .utf8)
    }
}

private struct ProjectFile: Encodable {
    let id: String
    let name: String
    let hierarchy: [ChiknTreeNode]
    let created: String
    let modified: String
}

private struct DocumentMetaFile: Encodable {
    let id: String
    let name: String
    let created: String
    let modified: String
    let parent_id: String?
    let label: String?
    let status: String?
    let keywords: [String]?
    let synopsis: String?
}
