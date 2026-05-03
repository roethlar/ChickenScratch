import Foundation

/// A reference inside a scene's `fields` map that no longer resolves —
/// the entity or thread it points to was deleted. Non-fatal; UIs surface
/// these as a soft warning so the user can clear them.
public struct DanglingRef: Sendable, Identifiable, Hashable {
    public let documentID: String
    public let documentName: String
    /// One of the novelist convention keys: `pov_character`, `location`,
    /// `characters_in_scene`, `threads`.
    public let field: String
    public let missingID: String

    public var id: String { "\(documentID)|\(field)|\(missingID)" }

    public init(documentID: String, documentName: String, field: String, missingID: String) {
        self.documentID = documentID
        self.documentName = documentName
        self.field = field
        self.missingID = missingID
    }
}

public enum References {
    /// Walk every document's fields map and report references to entities
    /// or threads that don't exist. Mirrors the Rust `validate_references`
    /// command in `src-tauri/src/commands/threads.rs`.
    ///
    /// Slug = filename stem under the entity folder. Thread id = the slug
    /// stored in `threads.yaml`.
    public static func validate(_ project: Project) -> [DanglingRef] {
        let characterSlugs: Set<String> = Set(
            project.documents.values
                .filter { $0.relativePath.hasPrefix("characters/") }
                .map(slugFromPath(_:))
        )
        let locationSlugs: Set<String> = Set(
            project.documents.values
                .filter { $0.relativePath.hasPrefix("locations/") }
                .map(slugFromPath(_:))
        )
        let threadIDs: Set<String> = Set(project.threads.map(\.id))

        var out: [DanglingRef] = []
        for doc in project.documents.values {
            check(doc: doc, key: "pov_character", set: characterSlugs, into: &out)
            check(doc: doc, key: "characters_in_scene", set: characterSlugs, into: &out)
            check(doc: doc, key: "location", set: locationSlugs, into: &out)
            check(doc: doc, key: "threads", set: threadIDs, into: &out)
        }
        return out
    }

    private static func check(
        doc: Document,
        key: String,
        set: Set<String>,
        into out: inout [DanglingRef]
    ) {
        guard let value = doc.meta.fields[key] else { return }
        switch value {
        case .string(let s):
            if !s.isEmpty && !set.contains(s) {
                out.append(DanglingRef(documentID: doc.id, documentName: doc.name, field: key, missingID: s))
            }
        case .array(let arr):
            for v in arr {
                if let s = v.asString, !s.isEmpty, !set.contains(s) {
                    out.append(DanglingRef(documentID: doc.id, documentName: doc.name, field: key, missingID: s))
                }
            }
        default:
            return
        }
    }

    private static func slugFromPath(_ doc: Document) -> String {
        let last = (doc.relativePath as NSString).lastPathComponent
        return (last as NSString).deletingPathExtension
    }
}
