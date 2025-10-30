import Foundation
import Yams

final class ChiknProjectLoader {
    private let decoder = YAMLDecoder()

    func loadProject(at url: URL) throws -> ChiknProject {
        let projectFileURL = url.appendingPathComponent("project.yaml")
        guard FileManager.default.fileExists(atPath: projectFileURL.path) else {
            throw ChiknProjectError.missingProjectFile
        }

        let yaml = try String(contentsOf: projectFileURL, encoding: .utf8)
        var projectFile = try decoder.decode(ProjectFile.self, from: yaml)

        let documents = try loadDocuments(
            for: projectFile.hierarchy,
            projectURL: url
        )

        return ChiknProject(
            id: projectFile.id,
            name: projectFile.name,
            created: DateParser.parse(projectFile.created),
            modified: DateParser.parse(projectFile.modified),
            url: url,
            hierarchy: projectFile.hierarchy,
            documents: documents
        )
    }

    private func loadDocuments(
        for nodes: [ChiknTreeNode],
        projectURL: URL
    ) throws -> [String: ChiknDocument] {
        var documents: [String: ChiknDocument] = [:]
        try traverse(nodes: nodes, projectURL: projectURL, storage: &documents)
        return documents
    }

    private func traverse(
        nodes: [ChiknTreeNode],
        projectURL: URL,
        storage: inout [String: ChiknDocument]
    ) throws {
        for node in nodes {
            switch node {
            case .folder(let folder):
                try traverse(nodes: folder.children, projectURL: projectURL, storage: &storage)
            case .document(let document):
                let docURL = projectURL.appendingPathComponent(document.path)
                guard FileManager.default.fileExists(atPath: docURL.path) else {
                    continue
                }

                let content = try String(contentsOf: docURL, encoding: .utf8)
                let metadata = try loadMetadata(for: document, projectURL: projectURL)

                let chiknDocument = ChiknDocument(
                    id: document.id,
                    name: document.name,
                    relativePath: document.path,
                    content: content,
                    metadata: metadata
                )
                storage[chiknDocument.id] = chiknDocument
            }
        }
    }

    private func loadMetadata(
        for document: ChiknTreeNode.DocumentNode,
        projectURL: URL
    ) throws -> DocumentMetadata {
        let metaPath = document.path.replacingOccurrences(of: ".md", with: ".meta")
        let metaURL = projectURL.appendingPathComponent(metaPath)

        guard FileManager.default.fileExists(atPath: metaURL.path) else {
            return DocumentMetadata(
                projectURL: projectURL,
                name: document.name,
                created: Date(),
                modified: Date(),
                parentID: nil,
                label: nil,
                status: nil,
                keywords: nil,
                synopsis: nil
            )
        }

        let metaContent = try String(contentsOf: metaURL, encoding: .utf8)
        let metaFile = try decoder.decode(DocumentMetaFile.self, from: metaContent)
        return DocumentMetadata(
            projectURL: projectURL,
            name: metaFile.name ?? document.name,
            created: DateParser.parse(metaFile.created),
            modified: DateParser.parse(metaFile.modified),
            parentID: metaFile.parent_id,
            label: metaFile.label,
            status: metaFile.status,
            keywords: metaFile.keywords,
            synopsis: metaFile.synopsis
        )
    }
}

enum ChiknProjectError: Error {
    case missingProjectFile
    case failedToWriteProject
}

private struct ProjectFile: Decodable {
    let id: String
    let name: String
    var hierarchy: [ChiknTreeNode]
    let created: String
    let modified: String
}

private struct DocumentMetaFile: Decodable {
    let id: String
    let name: String?
    let created: String
    let modified: String
    let parent_id: String?
    let label: String?
    let status: String?
    let keywords: [String]?
    let synopsis: String?
}
