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

        // Walk disk for all .md files under known roots — don't gate on
        // hierarchy presence, otherwise entities under characters/ and
        // locations/ (which Tauri doesn't list in project.yaml.hierarchy)
        // would silently disappear.
        let documents = try loadAllDocuments(in: url)

        let threads = readThreads(at: url)

        return Project(
            id: id,
            name: name,
            path: url,
            created: created,
            modified: modified,
            hierarchy: hierarchy,
            metadata: metadata,
            documents: documents,
            threads: threads
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
        let session = decodeSessionTarget(map["session_target"] as? [String: Any])
        return ProjectMetadata(
            title: s("title"),
            author: s("author"),
            projectType: s("project_type"),
            genre: s("genre"),
            theme: s("theme"),
            summary: s("summary"),
            sessionTarget: session
        )
    }

    private static func decodeSessionTarget(_ map: [String: Any]?) -> SessionTarget? {
        guard let map else { return nil }
        let target = SessionTarget(
            wordsPerSession: map["words_per_session"] as? Int,
            deadline: (map["deadline"] as? String).flatMap { $0.isEmpty ? nil : $0 },
            totalTarget: map["total_target"] as? Int
        )
        return target.isEmpty ? nil : target
    }

    // MARK: - Threads

    private static func readThreads(at projectURL: URL) -> [Thread] {
        let path = projectURL.appendingPathComponent("threads.yaml")
        guard FileManager.default.fileExists(atPath: path.path) else { return [] }
        guard let text = try? String(contentsOf: path, encoding: .utf8),
              !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              let parsed = (try? Yams.load(yaml: text)) as? [String: Any],
              let list = parsed["threads"] as? [Any]
        else { return [] }

        var out: [Thread] = []
        for raw in list {
            guard let map = raw as? [String: Any],
                  let id = map["id"] as? String,
                  let name = map["name"] as? String
            else { continue }
            let color = (map["color"] as? String).flatMap { $0.isEmpty ? nil : $0 }
            let description = (map["description"] as? String).flatMap { $0.isEmpty ? nil : $0 }
            out.append(Thread(id: id, name: name, color: color, description: description))
        }
        return out
    }

    // MARK: - Documents

    /// Roots scanned for .md documents. `manuscript`, `research`, `templates`
    /// are the regular content folders; `characters` and `locations` are the
    /// novelist-convention entity folders. Tauri's reader scans the same set.
    private static let documentRoots = [
        "manuscript", "research", "templates", "characters", "locations",
    ]

    private static func loadAllDocuments(in projectURL: URL) throws -> [String: Document] {
        var out: [String: Document] = [:]
        let fm = FileManager.default
        for root in documentRoots {
            let rootURL = projectURL.appendingPathComponent(root)
            guard fm.fileExists(atPath: rootURL.path) else { continue }
            guard let enumerator = fm.enumerator(at: rootURL, includingPropertiesForKeys: nil) else { continue }
            for case let fileURL as URL in enumerator where fileURL.pathExtension == "md" {
                if let doc = readDocument(at: fileURL, projectURL: projectURL) {
                    out[doc.id] = doc
                }
            }
        }
        return out
    }

    private static func readDocument(at fileURL: URL, projectURL: URL) -> Document? {
        let metaURL = fileURL.deletingPathExtension().appendingPathExtension("meta")
        guard let metaMap = readMetaMap(at: metaURL),
              let id = metaMap["id"] as? String
        else { return nil }

        let content = (try? String(contentsOf: fileURL, encoding: .utf8)) ?? ""
        let relative = relativePath(of: fileURL, in: projectURL)
        let name = (metaMap["name"] as? String).flatMap { $0.isEmpty ? nil : $0 }
                ?? fileURL.deletingPathExtension().lastPathComponent
        let meta = decodeDocumentMeta(metaMap)
        return Document(id: id, name: name, relativePath: relative, content: content, meta: meta)
    }

    private static func relativePath(of fileURL: URL, in projectURL: URL) -> String {
        // Resolve symlinks on both sides before stripping. macOS's
        // `temporaryDirectory` returns `/var/folders/...` (a symlink to
        // `/private/var/folders/...`) while `FileManager.enumerator` gives
        // back URLs with the resolved path — without normalizing, the prefix
        // strip silently fails and the document carries an absolute path
        // forever, which the writer then concatenates onto the project root.
        let absFile = fileURL.standardizedFileURL.resolvingSymlinksInPath().path
        let absProject = projectURL.standardizedFileURL.resolvingSymlinksInPath().path
        let prefix = absProject.hasSuffix("/") ? absProject : absProject + "/"
        if absFile.hasPrefix(prefix) {
            return String(absFile.dropFirst(prefix.count))
        }
        return absFile
    }

    private static func readMetaMap(at url: URL) -> [String: Any]? {
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        guard let text = try? String(contentsOf: url, encoding: .utf8) else { return nil }
        return (try? Yams.load(yaml: text)) as? [String: Any]
    }

    /// Top-level keys that map to typed columns on `DocumentMeta`. Anything
    /// outside this set inside `fields:` is preserved through `meta.fields`;
    /// foreign top-level keys are dropped on read (matches Tauri's behavior).
    private static let typedMetaKeys: Set<String> = [
        "id", "name", "created", "modified", "parent_id",
        "label", "status", "keywords", "synopsis",
        "section_type", "include_in_compile", "scrivener_uuid", "links",
        "word_count_target", "compile_order", "comments",
        "fields",
    ]

    private static func decodeDocumentMeta(_ map: [String: Any]) -> DocumentMeta {
        let keywords: [String]
        if let arr = map["keywords"] as? [String] {
            keywords = arr
        } else if let joined = map["keywords"] as? String {
            keywords = joined.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        } else {
            keywords = []
        }

        let includeInCompile: Bool
        if let b = map["include_in_compile"] as? Bool {
            includeInCompile = b
        } else if let s = map["include_in_compile"] as? String {
            // Scrivener-imported projects round-trip this as "Yes"/"No" strings.
            includeInCompile = s.lowercased() != "no"
        } else {
            includeInCompile = true
        }

        var fields: [String: YAMLValue] = [:]
        if let raw = map["fields"] as? [String: Any] {
            for (k, v) in raw {
                if let yv = YAMLValue(any: v) {
                    fields[k] = yv
                }
            }
        }

        return DocumentMeta(
            synopsis: (map["synopsis"] as? String).flatMap { $0.isEmpty ? nil : $0 },
            label: (map["label"] as? String).flatMap { $0.isEmpty ? nil : $0 },
            status: (map["status"] as? String).flatMap { $0.isEmpty ? nil : $0 },
            keywords: keywords,
            includeInCompile: includeInCompile,
            wordCountTarget: map["word_count_target"] as? Int,
            compileOrder: map["compile_order"] as? Int,
            fields: fields
        )
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
