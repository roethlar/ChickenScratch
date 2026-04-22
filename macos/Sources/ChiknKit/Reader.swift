import Foundation
import Yams

public enum Reader {
    public static func readProject(at url: URL) throws -> Project {
        let yamlURL = url.appendingPathComponent("project.yaml")
        guard FileManager.default.fileExists(atPath: yamlURL.path) else {
            throw ChiknError.notAChiknFolder(url)
        }

        let yaml = try String(contentsOf: yamlURL, encoding: .utf8)
        let root: Any
        do {
            guard let parsed = try Yams.load(yaml: yaml) else {
                throw ChiknError.invalidProjectYaml("project.yaml is empty")
            }
            root = parsed
        } catch let error as ChiknError {
            throw error
        } catch {
            throw ChiknError.invalidProjectYaml(error.localizedDescription)
        }

        guard let map = root as? [String: Any] else {
            throw ChiknError.invalidProjectYaml("expected a YAML mapping at the top level")
        }

        let id = (map["id"] as? String) ?? UUID().uuidString
        let name = (map["name"] as? String) ?? url.deletingPathExtension().lastPathComponent
        let created = parseDate(map["created"]) ?? Date()
        let modified = parseDate(map["modified"]) ?? created

        let hierarchy = decodeNodes(map["hierarchy"])
        let metadata = decodeMetadata(map["metadata"] as? [String: Any])

        let documents = try loadDocuments(for: hierarchy, in: url)

        return Project(
            id: id,
            name: name,
            path: url,
            created: created,
            modified: modified,
            hierarchy: hierarchy,
            metadata: metadata,
            documents: documents
        )
    }

    // MARK: - Hierarchy

    private static func decodeNodes(_ raw: Any?) -> [TreeNode] {
        guard let array = raw as? [Any] else { return [] }
        return array.compactMap(decodeNode)
    }

    private static func decodeNode(_ raw: Any) -> TreeNode? {
        guard let map = raw as? [String: Any],
              let id = map["id"] as? String,
              let name = map["name"] as? String,
              let typeString = map["type"] as? String,
              let kind = NodeKind(rawValue: typeString)
        else { return nil }

        let children = decodeNodes(map["children"])
        return TreeNode(id: id, name: name, kind: kind, children: children)
    }

    // MARK: - Metadata

    private static func decodeMetadata(_ map: [String: Any]?) -> ProjectMetadata {
        guard let map else { return ProjectMetadata() }
        func s(_ key: String) -> String? {
            let v = map[key] as? String
            return v?.isEmpty == true ? nil : v
        }
        return ProjectMetadata(
            title: s("title"),
            author: s("author"),
            projectType: s("project_type"),
            genre: s("genre"),
            theme: s("theme"),
            summary: s("summary")
        )
    }

    // MARK: - Documents

    private static func loadDocuments(for nodes: [TreeNode], in projectURL: URL) throws -> [String: Document] {
        var out: [String: Document] = [:]
        for node in nodes {
            try walk(node: node, projectURL: projectURL, into: &out)
        }
        return out
    }

    private static func walk(node: TreeNode, projectURL: URL, into out: inout [String: Document]) throws {
        if node.kind == .document {
            if let doc = try loadDocument(id: node.id, name: node.name, projectURL: projectURL) {
                out[node.id] = doc
            }
        }
        for child in node.children {
            try walk(node: child, projectURL: projectURL, into: &out)
        }
    }

    private static func loadDocument(id: String, name: String, projectURL: URL) throws -> Document? {
        // Search manuscript/, research/, templates/ for a matching .md file by id or slug.
        // The YAML's `path` field is authoritative when present; fall back to a filename search.
        let roots = ["manuscript", "research", "templates"]
        let fm = FileManager.default

        for root in roots {
            let rootURL = projectURL.appendingPathComponent(root)
            guard let enumerator = fm.enumerator(at: rootURL, includingPropertiesForKeys: nil) else { continue }
            for case let fileURL as URL in enumerator where fileURL.pathExtension == "md" {
                let metaURL = fileURL.deletingPathExtension().appendingPathExtension("meta")
                guard let meta = try? readMeta(at: metaURL), meta.id == id else { continue }

                let content = (try? String(contentsOf: fileURL, encoding: .utf8)) ?? ""
                let relative = fileURL.path.replacingOccurrences(of: projectURL.path + "/", with: "")
                return Document(id: id, name: name, relativePath: relative, content: content, meta: meta.doc)
            }
        }
        return nil
    }

    private struct MetaRow {
        let id: String
        let doc: DocumentMeta
    }

    private static func readMeta(at url: URL) throws -> MetaRow? {
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        let text = try String(contentsOf: url, encoding: .utf8)
        guard let parsed = try Yams.load(yaml: text) as? [String: Any],
              let id = parsed["id"] as? String
        else { return nil }

        let keywords: [String]
        if let arr = parsed["keywords"] as? [String] {
            keywords = arr
        } else if let joined = parsed["keywords"] as? String {
            keywords = joined.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        } else {
            keywords = []
        }

        let doc = DocumentMeta(
            synopsis: parsed["synopsis"] as? String,
            label: parsed["label"] as? String,
            status: parsed["status"] as? String,
            keywords: keywords,
            includeInCompile: (parsed["include_in_compile"] as? Bool) ?? true,
            wordCountTarget: parsed["word_count_target"] as? Int,
            compileOrder: parsed["compile_order"] as? Int
        )
        return MetaRow(id: id, doc: doc)
    }

    // MARK: - Dates

    private static func parseDate(_ raw: Any?) -> Date? {
        if let d = raw as? Date { return d }
        guard let s = raw as? String else { return nil }
        let withFractional = Date.ISO8601FormatStyle(includingFractionalSeconds: true)
        let plain = Date.ISO8601FormatStyle()
        return (try? withFractional.parse(s)) ?? (try? plain.parse(s))
    }
}
