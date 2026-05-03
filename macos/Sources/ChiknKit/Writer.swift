import Foundation
import Yams

public enum Writer {
    // MARK: - Public API

    /// Create a new .chikn project folder at `path` with `name`.
    /// Writes project.yaml, creates manuscript/research/trash folders,
    /// inits git, and makes an initial commit. Returns the new Project.
    public static func createProject(at projectURL: URL, name: String) throws -> Project {
        let fm = FileManager.default
        try fm.createDirectory(at: projectURL, withIntermediateDirectories: true)
        for folder in ["manuscript", "research", "trash"] {
            try fm.createDirectory(at: projectURL.appendingPathComponent(folder), withIntermediateDirectories: true)
        }
        let id = UUID().uuidString.lowercased()
        let now = Date()
        let project = Project(
            id: id,
            name: name,
            path: projectURL,
            created: now,
            modified: now,
            hierarchy: [
                TreeNode(id: "manuscript", name: "Manuscript", kind: .folder),
                TreeNode(id: "research",   name: "Research",   kind: .folder),
                TreeNode(id: "trash",      name: "Trash",      kind: .folder),
            ],
            metadata: ProjectMetadata(),
            documents: [:]
        )
        try writeProjectYaml(project)
        try Git.initRepoIfNeeded(at: projectURL)
        try? Git.saveRevision(message: "Initial commit", in: projectURL)
        return project
    }

    /// Update the metadata fields in a document's .meta sidecar and project.yaml.
    /// Returns the updated Project.
    @discardableResult
    public static func saveDocumentMeta(
        id: String,
        meta: DocumentMeta,
        in project: Project
    ) throws -> Project {
        guard var doc = project.documents[id] else {
            throw ChiknError.documentMissing(id)
        }
        doc.meta = meta
        var project = project
        project.documents[id] = doc
        project.modified = Date()

        let docURL = project.path.appendingPathComponent(doc.relativePath)
        let metaURL = docURL.deletingPathExtension().appendingPathExtension("meta")

        var metaMap: [String: Any] = [:]
        if let existing = try? loadYaml(metaURL) { metaMap = existing ?? [:] }
        if let v = meta.synopsis          { metaMap["synopsis"] = v }              else { metaMap.removeValue(forKey: "synopsis") }
        if let v = meta.label             { metaMap["label"] = v }                 else { metaMap.removeValue(forKey: "label") }
        if let v = meta.status            { metaMap["status"] = v }                else { metaMap.removeValue(forKey: "status") }
        if !meta.keywords.isEmpty         { metaMap["keywords"] = meta.keywords }  else { metaMap.removeValue(forKey: "keywords") }
        metaMap["include_in_compile"] = meta.includeInCompile
        if let wt = meta.wordCountTarget  { metaMap["word_count_target"] = wt }    else { metaMap.removeValue(forKey: "word_count_target") }
        if let co = meta.compileOrder     { metaMap["compile_order"] = co }        else { metaMap.removeValue(forKey: "compile_order") }

        // `fields` is the format's sole UI-extensibility point. Replace the
        // entire block — empty map drops the `fields:` key entirely so .meta
        // files stay clean for projects that don't use it.
        if meta.fields.isEmpty {
            metaMap.removeValue(forKey: "fields")
        } else {
            var fieldDict: [String: Any] = [:]
            for (k, v) in meta.fields { fieldDict[k] = v.toAny() }
            metaMap["fields"] = fieldDict
        }

        metaMap["modified"] = iso8601Now()
        try writeYaml(metaMap, to: metaURL)

        try writeProjectYaml(project)
        return project
    }

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

    /// Create a character or location entity. Entities live under `characters/`
    /// or `locations/` as regular Documents — the format itself stays
    /// genre-agnostic, the kind is tagged via `fields["entity_kind"]` so any UI
    /// (or convention reader) can detect it. They are NOT added to
    /// project.yaml.hierarchy; the binder surfaces them by walking
    /// `project.documents` under the relevant path prefix.
    @discardableResult
    public static func createEntity(
        kind: EntityKind,
        name: String,
        in project: Project
    ) throws -> (Project, Document) {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw ChiknError.invalidProjectYaml("Entity name cannot be empty")
        }

        let folder = kind.folderName
        let folderURL = project.path.appendingPathComponent(folder)
        try FileManager.default.createDirectory(at: folderURL, withIntermediateDirectories: true)

        let slug = uniqueSlug(for: trimmed, in: folder, projectPath: project.path)
        let relativePath = "\(folder)/\(slug).md"
        let absoluteURL = project.path.appendingPathComponent(relativePath)

        let id = UUID().uuidString.lowercased()
        let now = iso8601Now()

        try "".write(to: absoluteURL, atomically: true, encoding: .utf8)

        let metaURL = absoluteURL.deletingPathExtension().appendingPathExtension("meta")
        let meta: [String: Any] = [
            "id": id,
            "name": trimmed,
            "created": now,
            "modified": now,
            "fields": ["entity_kind": kind.rawValue],
        ]
        try writeYaml(meta, to: metaURL)

        var project = project
        let document = Document(
            id: id,
            name: trimmed,
            relativePath: relativePath,
            content: "",
            meta: DocumentMeta(fields: ["entity_kind": .string(kind.rawValue)])
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

    /// Remove a node from the hierarchy and delete its files. For folders,
    /// recurses into children — every contained document loses its `.md` and
    /// `.meta` file. The node is also dropped from `project.documents`.
    /// Match Tauri `delete_node` semantics: no Trash relocation, the caller
    /// is expected to have moved into Trash first if soft-delete was wanted.
    @discardableResult
    public static func deleteNode(id: String, in project: Project) throws -> Project {
        var project = project
        let removed = removeNode(id: id, from: &project.hierarchy)
        if let removed {
            try deleteNodeFiles(removed, in: &project)
        }
        project.modified = Date()
        try writeProjectYaml(project)
        return project
    }

    /// Move a node to a new parent (`nil` = root) and optionally to a
    /// specific index within that parent's children. Files on disk stay put;
    /// only the hierarchy is updated.
    @discardableResult
    public static func moveNode(
        id: String,
        newParentID: String?,
        newIndex: Int? = nil,
        in project: Project
    ) throws -> Project {
        var project = project
        guard let node = removeNode(id: id, from: &project.hierarchy) else {
            throw ChiknError.documentMissing("Node \(id)")
        }
        project.hierarchy = insertNode(node, parentID: newParentID, in: project.hierarchy, atIndex: newIndex)
        project.modified = Date()
        try writeProjectYaml(project)
        return project
    }

    /// Move a node within its current parent to a new index (used for
    /// up/down reordering in the binder).
    @discardableResult
    public static func reorderNode(id: String, newIndex: Int, in project: Project) throws -> Project {
        var project = project
        let parentID = findParent(of: id, in: project.hierarchy, currentParent: nil)
        guard let node = removeNode(id: id, from: &project.hierarchy) else {
            throw ChiknError.documentMissing("Node \(id)")
        }
        project.hierarchy = insertNode(node, parentID: parentID, in: project.hierarchy, atIndex: newIndex)
        project.modified = Date()
        try writeProjectYaml(project)
        return project
    }

    private static func removeNode(id: String, from nodes: inout [TreeNode]) -> TreeNode? {
        if let idx = nodes.firstIndex(where: { $0.id == id }) {
            return nodes.remove(at: idx)
        }
        for i in nodes.indices {
            var children = nodes[i].children
            if let removed = removeNode(id: id, from: &children) {
                nodes[i].children = children
                return removed
            }
        }
        return nil
    }

    private static func findParent(of id: String, in nodes: [TreeNode], currentParent: String?) -> String? {
        for node in nodes {
            if node.id == id { return currentParent }
            if let found = findParent(of: id, in: node.children, currentParent: node.id) {
                return found
            }
        }
        return nil
    }

    private static func deleteNodeFiles(_ node: TreeNode, in project: inout Project) throws {
        switch node.kind {
        case .document:
            if let doc = project.documents[node.id] {
                let docURL = project.path.appendingPathComponent(doc.relativePath)
                let metaURL = docURL.deletingPathExtension().appendingPathExtension("meta")
                try? FileManager.default.removeItem(at: docURL)
                try? FileManager.default.removeItem(at: metaURL)
                project.documents.removeValue(forKey: node.id)
            }
        case .folder:
            for child in node.children {
                try deleteNodeFiles(child, in: &project)
            }
        }
    }

    // MARK: - Threads

    @discardableResult
    public static func createThread(
        name: String,
        color: String? = nil,
        description: String? = nil,
        in project: Project
    ) throws -> Project {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw ChiknError.invalidProjectYaml("Thread name cannot be empty")
        }

        var project = project
        let id = uniqueThreadID(name: trimmed, existing: project.threads)
        project.threads.append(Thread(id: id, name: trimmed, color: color, description: description))
        project.modified = Date()
        try writeThreadsYaml(project)
        try writeProjectYaml(project)
        return project
    }

    @discardableResult
    public static func updateThread(
        id: String,
        name: String? = nil,
        color: String?? = nil,
        description: String?? = nil,
        in project: Project
    ) throws -> Project {
        var project = project
        guard let idx = project.threads.firstIndex(where: { $0.id == id }) else {
            throw ChiknError.documentMissing("Thread \(id)")
        }
        var thread = project.threads[idx]
        if let n = name?.trimmingCharacters(in: .whitespacesAndNewlines), !n.isEmpty {
            thread.name = n
        }
        if let c = color {
            thread.color = c?.isEmpty == true ? nil : c
        }
        if let d = description {
            thread.description = d?.isEmpty == true ? nil : d
        }
        project.threads[idx] = thread
        project.modified = Date()
        try writeThreadsYaml(project)
        try writeProjectYaml(project)
        return project
    }

    /// Delete a thread. Strips the ref from every scene's `fields["threads"]`
    /// list so we don't leave dangling references behind.
    @discardableResult
    public static func deleteThread(id: String, in project: Project) throws -> Project {
        var project = project
        project.threads.removeAll(where: { $0.id == id })

        for (docID, var doc) in project.documents {
            if case .array(let arr) = doc.meta.fields["threads"] {
                let filtered = arr.filter { $0.asString != id }
                if filtered.count != arr.count {
                    if filtered.isEmpty {
                        doc.meta.fields.removeValue(forKey: "threads")
                    } else {
                        doc.meta.fields["threads"] = .array(filtered)
                    }
                    project.documents[docID] = doc
                    // Persist the per-doc change through saveDocumentMeta to
                    // refresh the .meta file too.
                    project = try saveDocumentMeta(id: docID, meta: doc.meta, in: project)
                }
            }
        }

        project.modified = Date()
        try writeThreadsYaml(project)
        try writeProjectYaml(project)
        return project
    }

    private static func uniqueThreadID(name: String, existing: [Thread]) -> String {
        let base = slugify(name)
        let stem = base.isEmpty ? "thread" : base
        if !existing.contains(where: { $0.id == stem }) { return stem }
        var n = 2
        while existing.contains(where: { $0.id == "\(stem)-\(n)" }) { n += 1 }
        return "\(stem)-\(n)"
    }

    /// Write `threads.yaml` (or remove it if the project has no threads).
    private static func writeThreadsYaml(_ project: Project) throws {
        let url = project.path.appendingPathComponent("threads.yaml")
        if project.threads.isEmpty {
            // Avoid leaving a stale empty file behind after the user deletes
            // their last thread.
            if FileManager.default.fileExists(atPath: url.path) {
                try? FileManager.default.removeItem(at: url)
            }
            return
        }
        let payload: [String: Any] = [
            "threads": project.threads.map { thread -> [String: Any] in
                var entry: [String: Any] = [
                    "id": thread.id,
                    "name": thread.name,
                ]
                if let color = thread.color { entry["color"] = color }
                if let desc = thread.description { entry["description"] = desc }
                return entry
            },
        ]
        try writeYaml(payload, to: url)
    }

    // MARK: - Hierarchy helpers

    private static func insertNode(
        _ node: TreeNode,
        parentID: String?,
        in nodes: [TreeNode],
        path: String
    ) -> [TreeNode] {
        return insertNode(node, parentID: parentID, in: nodes, atIndex: nil)
    }

    private static func insertNode(
        _ node: TreeNode,
        parentID: String?,
        in nodes: [TreeNode],
        atIndex index: Int?
    ) -> [TreeNode] {
        // Append at root when no parent.
        guard let parentID else {
            var copy = nodes
            let clamped = clampIndex(index, count: copy.count)
            copy.insert(node, at: clamped)
            return copy
        }
        return nodes.map { existing in
            if existing.id == parentID {
                var updated = existing
                let clamped = clampIndex(index, count: updated.children.count)
                updated.children.insert(node, at: clamped)
                return updated
            }
            var updated = existing
            updated.children = insertNode(node, parentID: parentID, in: existing.children, atIndex: index)
            return updated
        }
    }

    private static func clampIndex(_ index: Int?, count: Int) -> Int {
        guard let i = index else { return count }
        return max(0, min(i, count))
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
        let sessionTarget: SessionTargetYaml?

        enum CodingKeys: String, CodingKey {
            case title, author, genre, theme, summary
            case projectType = "project_type"
            case sessionTarget = "session_target"
        }

        init(_ m: ProjectMetadata) {
            title = m.title ?? ""
            author = m.author ?? ""
            projectType = m.projectType ?? ""
            genre = m.genre ?? ""
            theme = m.theme ?? ""
            summary = m.summary ?? ""
            sessionTarget = m.sessionTarget.flatMap { $0.isEmpty ? nil : SessionTargetYaml($0) }
        }

        func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(title, forKey: .title)
            try c.encode(author, forKey: .author)
            try c.encode(projectType, forKey: .projectType)
            try c.encode(genre, forKey: .genre)
            try c.encode(theme, forKey: .theme)
            try c.encode(summary, forKey: .summary)
            try c.encodeIfPresent(sessionTarget, forKey: .sessionTarget)
        }
    }

    private struct SessionTargetYaml: Encodable {
        let wordsPerSession: Int?
        let deadline: String?
        let totalTarget: Int?

        enum CodingKeys: String, CodingKey {
            case wordsPerSession = "words_per_session"
            case deadline
            case totalTarget = "total_target"
        }

        init(_ t: SessionTarget) {
            wordsPerSession = t.wordsPerSession
            deadline = t.deadline
            totalTarget = t.totalTarget
        }

        func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encodeIfPresent(wordsPerSession, forKey: .wordsPerSession)
            try c.encodeIfPresent(deadline, forKey: .deadline)
            try c.encodeIfPresent(totalTarget, forKey: .totalTarget)
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

/// Entity kind tag used in `fields["entity_kind"]`.
public enum EntityKind: String, Sendable {
    case character
    case location

    public var folderName: String {
        switch self {
        case .character: "characters"
        case .location: "locations"
        }
    }
}
