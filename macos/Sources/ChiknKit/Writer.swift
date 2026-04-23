import Foundation
import Yams

public enum Writer {
    // MARK: - Public API

    /// Write a document's content back to disk. Updates the .meta modified
    /// timestamp and the project's modified timestamp.
    public static func saveDocument(_ document: Document, in project: Project) throws {
        let documentURL = project.path.appendingPathComponent(document.relativePath)
        try document.content.write(to: documentURL, atomically: true, encoding: .utf8)

        let metaURL = documentURL.deletingPathExtension().appendingPathExtension("meta")
        try touchMeta(metaURL)
        try touchProject(project)
    }

    /// Create a new empty document under `parentID` (nil = root). Returns an
    /// updated Project with the new document added and project.yaml rewritten.
    @discardableResult
    public static func createDocument(
        name: String,
        parentID: String?,
        in project: Project
    ) throws -> (Project, Document) {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw ChiknError.invalidProjectYaml("Document name cannot be empty")
        }

        let rootDir = rootDirectory(for: parentID, in: project.hierarchy)
        let slug = uniqueSlug(for: trimmed, in: rootDir, projectPath: project.path)
        let relativePath = "\(rootDir)/\(slug).md"
        let absoluteURL = project.path.appendingPathComponent(relativePath)

        try FileManager.default.createDirectory(
            at: absoluteURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )

        let id = UUID().uuidString.lowercased()
        let now = iso8601Now()

        try "".write(to: absoluteURL, atomically: true, encoding: .utf8)

        let metaURL = absoluteURL.deletingPathExtension().appendingPathExtension("meta")
        let meta: [String: Any] = [
            "id": id,
            "name": trimmed,
            "created": now,
            "modified": now,
            "parent_id": parentID as Any? as Any,
        ].compactMapValues { $0 is NSNull ? nil : $0 }
        try writeYaml(meta, to: metaURL)

        let node = TreeNode(id: id, name: trimmed, kind: .document)
        var project = project
        project.hierarchy = insertNode(node, parentID: parentID, in: project.hierarchy, path: relativePath)
        let document = Document(
            id: id,
            name: trimmed,
            relativePath: relativePath,
            content: "",
            meta: DocumentMeta()
        )
        project.documents[id] = document
        project.modified = Date()

        try writeProjectYaml(project)
        return (project, document)
    }

    /// Rename a node (document or folder) and persist. Updates project.yaml
    /// and — for documents — the .meta sidecar. Does not move files on disk;
    /// the .md filename is kept stable (the name lives in .meta + hierarchy).
    @discardableResult
    public static func renameNode(
        id: String,
        newName: String,
        in project: Project
    ) throws -> Project {
        let trimmed = newName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw ChiknError.invalidProjectYaml("Name cannot be empty")
        }

        var project = project
        project.hierarchy = renamingNode(id: id, to: trimmed, in: project.hierarchy)

        if var doc = project.documents[id] {
            doc.name = trimmed
            project.documents[id] = doc

            let metaURL = project.path
                .appendingPathComponent(doc.relativePath)
                .deletingPathExtension()
                .appendingPathExtension("meta")
            if var metaMap = try loadYaml(metaURL) {
                metaMap["name"] = trimmed
                metaMap["modified"] = iso8601Now()
                try writeYaml(metaMap, to: metaURL)
            }
        }

        project.modified = Date()
        try writeProjectYaml(project)
        return project
    }

    /// Touch project.yaml's modified timestamp and rewrite.
    public static func touchProject(_ project: Project) throws {
        var project = project
        project.modified = Date()
        try writeProjectYaml(project)
    }

    // MARK: - Hierarchy helpers

    private static func insertNode(
        _ node: TreeNode,
        parentID: String?,
        in nodes: [TreeNode],
        path: String
    ) -> [TreeNode] {
        // Append at root when no parent.
        guard let parentID else {
            return nodes + [node]
        }
        return nodes.map { existing in
            if existing.id == parentID {
                var updated = existing
                updated.children.append(node)
                return updated
            }
            var updated = existing
            updated.children = insertNode(node, parentID: parentID, in: existing.children, path: path)
            return updated
        }
    }

    private static func renamingNode(id: String, to newName: String, in nodes: [TreeNode]) -> [TreeNode] {
        nodes.map { existing in
            if existing.id == id {
                var updated = existing
                updated.name = newName
                updated.children = renamingNode(id: id, to: newName, in: existing.children)
                return updated
            }
            var updated = existing
            updated.children = renamingNode(id: id, to: newName, in: existing.children)
            return updated
        }
    }

    /// Find which top-level folder a parent lives under, so new children land
    /// next to their siblings on disk. Falls back to manuscript/.
    private static func rootDirectory(for parentID: String?, in hierarchy: [TreeNode]) -> String {
        guard let parentID else { return "manuscript" }
        if ["research", "trash", "templates"].contains(parentID) { return parentID }
        return findRoot(parentID: parentID, in: hierarchy, current: nil) ?? "manuscript"
    }

    private static func findRoot(parentID: String, in nodes: [TreeNode], current: String?) -> String? {
        for node in nodes {
            let here = current ?? (["manuscript", "research", "trash", "templates"].contains(node.id) ? node.id : nil)
            if node.id == parentID { return here ?? "manuscript" }
            if let found = findRoot(parentID: parentID, in: node.children, current: here) {
                return found
            }
        }
        return nil
    }

    // MARK: - Slug

    private static func uniqueSlug(for name: String, in folder: String, projectPath: URL) -> String {
        let base = slugify(name)
        let folderURL = projectPath.appendingPathComponent(folder)
        var candidate = base
        var counter = 2
        while FileManager.default.fileExists(atPath: folderURL.appendingPathComponent("\(candidate).md").path) {
            candidate = "\(base)-\(counter)"
            counter += 1
        }
        return candidate
    }

    private static func slugify(_ input: String) -> String {
        let lower = input.lowercased()
        var out = ""
        var prevHyphen = false
        for ch in lower {
            if ch.isLetter || ch.isNumber {
                out.append(ch)
                prevHyphen = false
            } else if !prevHyphen {
                out.append("-")
                prevHyphen = true
            }
        }
        while out.hasPrefix("-") { out.removeFirst() }
        while out.hasSuffix("-") { out.removeLast() }
        return out.isEmpty ? "document" : out
    }

    // MARK: - Meta / project.yaml I/O

    private static func touchMeta(_ url: URL) throws {
        guard FileManager.default.fileExists(atPath: url.path) else { return }
        guard var map = try loadYaml(url) else { return }
        map["modified"] = iso8601Now()
        try writeYaml(map, to: url)
    }

    private static func loadYaml(_ url: URL) throws -> [String: Any]? {
        let text = try String(contentsOf: url, encoding: .utf8)
        return try Yams.load(yaml: text) as? [String: Any]
    }

    private static func writeYaml(_ map: [String: Any], to url: URL) throws {
        let text = try Yams.dump(object: map)
        try text.write(to: url, atomically: true, encoding: .utf8)
    }

    private static func writeProjectYaml(_ project: Project) throws {
        let url = project.path.appendingPathComponent("project.yaml")
        let payload = ProjectYaml(project: project)
        let encoder = YAMLEncoder()
        let text = try encoder.encode(payload)
        try text.write(to: url, atomically: true, encoding: .utf8)
    }

    // MARK: - Wire structs (preserve key order for clean git diffs)

    private struct ProjectYaml: Encodable {
        let id: String
        let name: String
        let created: String
        let modified: String
        let metadata: MetadataYaml
        let hierarchy: [NodeYaml]

        init(project: Project) {
            id = project.id
            name = project.name
            created = Writer.iso8601(project.created)
            modified = Writer.iso8601(project.modified)
            metadata = MetadataYaml(project.metadata)
            hierarchy = project.hierarchy.map { NodeYaml($0, in: project) }
        }
    }

    private struct MetadataYaml: Encodable {
        let title: String
        let author: String
        let projectType: String
        let genre: String
        let theme: String
        let summary: String

        enum CodingKeys: String, CodingKey {
            case title, author, genre, theme, summary
            case projectType = "project_type"
        }

        init(_ m: ProjectMetadata) {
            title = m.title ?? ""
            author = m.author ?? ""
            projectType = m.projectType ?? ""
            genre = m.genre ?? ""
            theme = m.theme ?? ""
            summary = m.summary ?? ""
        }
    }

    private struct NodeYaml: Encodable {
        let id: String
        let name: String
        let type: String
        let path: String?
        let children: [NodeYaml]

        init(_ node: TreeNode, in project: Project) {
            id = node.id
            name = node.name
            type = node.kind.rawValue
            if node.kind == .document {
                path = project.documents[node.id]?.relativePath
            } else {
                path = nil
            }
            children = node.children.map { NodeYaml($0, in: project) }
        }
    }

    // MARK: - Dates

    private static func iso8601Now() -> String {
        iso8601(Date())
    }

    private static func iso8601(_ date: Date) -> String {
        Date.ISO8601FormatStyle(includingFractionalSeconds: true).format(date)
    }
}
