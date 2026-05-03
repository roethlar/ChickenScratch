import AppKit
import ChiknKit
import Observation
import SwiftUI

@Observable
@MainActor
final class ProjectStore {
    var project: Project?
    var selectedNodeID: TreeNode.ID?
    var showInspector: Bool = true
    var errorMessage: String?
    var saveState: SaveState = .saved
    var recentProjects: [RecentProject] = []

    /// Auto-commit threshold. After this many seconds since the last commit,
    /// the next saveDocument triggers an "Auto: <timestamp>" revision.
    private let autoCommitInterval: TimeInterval = 10 * 60
    private var lastAutoCommit: Date?

    enum SaveState: Equatable {
        case saved
        case dirty
        case saving
        case failed(String)
    }

    struct RecentProject: Identifiable, Codable, Sendable {
        var id: String { path }
        let name: String
        let path: String
    }

    init() {
        recentProjects = loadRecents()
    }

    // MARK: - Open / close

    func openPickedProject() {
        let panel = NSOpenPanel()
        panel.title = "Open ChickenScratch Project"
        panel.canChooseFiles = true
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.treatsFilePackagesAsDirectories = false
        if panel.runModal() == .OK, let url = panel.url {
            open(url: url)
        }
    }

    func open(url: URL) {
        do {
            let loaded = try Reader.readProject(at: url)
            project = loaded
            selectedNodeID = firstDocumentID(in: loaded.hierarchy)
            errorMessage = nil
            saveState = .saved
            lastAutoCommit = nil
            try? Git.initRepoIfNeeded(at: loaded.path)
            addToRecent(name: loaded.name, path: url.path)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func closeProject() {
        project = nil
        selectedNodeID = nil
        saveState = .saved
    }

    func createProject(name: String, at url: URL) {
        do {
            let p = try Writer.createProject(at: url, name: name)
            project = p
            selectedNodeID = nil
            saveState = .saved
            errorMessage = nil
            addToRecent(name: name, path: url.path)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Recent projects

    func addToRecent(name: String, path: String) {
        var recents = loadRecents()
        recents.removeAll { $0.path == path }
        recents.insert(RecentProject(name: name, path: path), at: 0)
        if recents.count > 10 { recents = Array(recents.prefix(10)) }
        if let data = try? JSONEncoder().encode(recents) {
            UserDefaults.standard.set(data, forKey: "recentProjects")
        }
        recentProjects = recents
    }

    func loadRecents() -> [RecentProject] {
        guard let data = UserDefaults.standard.data(forKey: "recentProjects"),
              let list = try? JSONDecoder().decode([RecentProject].self, from: data)
        else { return [] }
        return list
    }

    // MARK: - Document I/O

    func activeDocument() -> Document? {
        guard let project, let id = selectedNodeID else { return nil }
        return project.documents[id]
    }

    /// Persist `content` for document `id`. Caller typically debounces by
    /// waiting for the user to pause typing; this method writes unconditionally.
    func saveDocument(id: String, content: String) {
        guard var p = project, var doc = p.documents[id] else { return }
        if doc.content == content {
            saveState = .saved
            return
        }
        saveState = .saving
        doc.content = content
        p.documents[id] = doc
        do {
            try Writer.saveDocument(doc, in: p)
            p.modified = Date()
            project = p
            saveState = .saved
            maybeAutoCommit()
        } catch {
            saveState = .failed(error.localizedDescription)
            errorMessage = "Couldn't save: \(error.localizedDescription)"
        }
    }

    func saveDocumentMeta(id: String, meta: DocumentMeta) {
        guard let p = project else { return }
        do {
            project = try Writer.saveDocumentMeta(id: id, meta: meta, in: p)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Entities (characters, locations)

    /// Create a character or location entity. Entities live as plain Documents
    /// under `characters/` or `locations/` and are tagged via `entity_kind` in
    /// their fields map. Returns the new document so the caller can select it.
    @discardableResult
    func createEntity(kind: EntityKind, name: String) -> Document? {
        guard let p = project else { return nil }
        do {
            let (updated, doc) = try Writer.createEntity(kind: kind, name: name, in: p)
            project = updated
            return doc
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }

    /// All loaded documents whose path lives under the entity folder for `kind`,
    /// sorted by name. The Binder's entity section uses this directly because
    /// entities aren't in `project.yaml.hierarchy`.
    func entities(of kind: EntityKind) -> [Document] {
        guard let p = project else { return [] }
        let prefix = kind.folderName + "/"
        return p.documents.values
            .filter { $0.relativePath.hasPrefix(prefix) }
            .sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    // MARK: - Threads

    func createThread(name: String, color: String? = nil) {
        guard let p = project else { return }
        do {
            project = try Writer.createThread(name: name, color: color, in: p)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func updateThread(id: String, name: String? = nil, color: String?? = nil, description: String?? = nil) {
        guard let p = project else { return }
        do {
            project = try Writer.updateThread(id: id, name: name, color: color, description: description, in: p)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func deleteThread(id: String) {
        guard let p = project else { return }
        do {
            project = try Writer.deleteThread(id: id, in: p)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Drafts (branches)

    /// Create a new draft branched off HEAD and switch to it. The current
    /// working tree is committed first if dirty so the user doesn't lose
    /// in-flight edits across the branch boundary.
    func createDraft(name: String) async {
        guard let p = project else { return }
        await commitIfDirty(message: "Auto: pre-draft snapshot", in: p)
        do {
            try Git.createDraft(name: name, in: p.path)
            await reloadAfterGit(p.path)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    /// All drafts, with the active one marked. UI-side cache; refresh by
    /// calling this whenever the Revisions panel becomes visible or after a
    /// switch/merge/create.
    func listDrafts() -> [Git.DraftVersion] {
        guard let url = project?.path else { return [] }
        return (try? Git.listDrafts(in: url)) ?? []
    }

    /// Switch to `name`. Auto-commits dirty state first. `git checkout` is
    /// force-style so any uncommitted edits we don't preserve would be lost
    /// — the pre-switch commit is the safety net.
    func switchDraft(name: String) async {
        guard let p = project else { return }
        await commitIfDirty(message: "Auto: pre-switch snapshot", in: p)
        do {
            try Git.switchDraft(name: name, in: p.path)
            await reloadAfterGit(p.path)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func mergeDraft(name: String) async {
        guard let p = project else { return }
        await commitIfDirty(message: "Auto: pre-merge snapshot", in: p)
        do {
            try Git.mergeDraft(name: name, in: p.path)
            await reloadAfterGit(p.path)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Per-document history

    func documentHistory(for documentPath: String) -> [Git.RevisionEntry] {
        guard let url = project?.path else { return [] }
        return (try? Git.documentHistory(documentPath: documentPath, in: url)) ?? []
    }

    func restoreDocument(documentPath: String, commit: String) async {
        guard let p = project else { return }
        await commitIfDirty(message: "Auto: pre-restore snapshot", in: p)
        do {
            try Git.restoreDocument(documentPath: documentPath, commitHash: commit, in: p.path)
            await reloadAfterGit(p.path)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Dangling refs

    func danglingReferences() -> [DanglingRef] {
        guard let p = project else { return [] }
        return References.validate(p)
    }

    // MARK: - Stats / writing history / session progress

    func projectStats() -> ProjectStats? {
        guard let p = project else { return nil }
        return Stats.projectStats(p)
    }

    func sessionProgress() -> SessionProgress? {
        guard let p = project else { return nil }
        return Stats.sessionProgress(p)
    }

    func writingHistory() -> WritingHistory {
        guard let p = project else { return WritingHistory() }
        return Stats.loadWritingHistory(in: p.path)
    }

    /// Record the current manuscript word count for today. Best called
    /// whenever the Stats panel becomes visible — that's when we can show
    /// today's progress accurately.
    func recordDailyWordsNow() {
        guard let p = project else { return }
        let stats = Stats.projectStats(p)
        _ = try? Stats.recordDailyWords(stats.manuscriptWords, in: p.path)
    }

    /// Replace the project's session target. nil clears it; an empty target
    /// is also normalized to nil so project.yaml stays clean.
    func updateSessionTarget(_ target: SessionTarget?) {
        guard var p = project else { return }
        if let t = target, !t.isEmpty {
            p.metadata.sessionTarget = t
        } else {
            p.metadata.sessionTarget = nil
        }
        do {
            try Writer.touchProject(p)
            project = p
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    // MARK: - Binder operations (delete / move / trash)

    /// Move a node to the project's Trash folder. If the node is already
    /// inside Trash, calls `deleteNode` instead (permanent delete).
    func deleteNode(id: String) {
        guard let p = project else { return }
        let trashID = p.hierarchy.first(where: { $0.kind == .folder && $0.name == "Trash" })?.id

        do {
            if let trashID, !isDescendant(of: trashID, id: id, in: p.hierarchy) {
                project = try Writer.moveNode(id: id, newParentID: trashID, in: p)
            } else {
                project = try Writer.deleteNode(id: id, in: p)
                if selectedNodeID == id {
                    selectedNodeID = nil
                }
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    /// Permanent delete — used by "Empty Trash" and direct "Delete" calls
    /// when the caller wants to skip the Trash step.
    func deleteNodePermanently(id: String) {
        guard let p = project else { return }
        do {
            project = try Writer.deleteNode(id: id, in: p)
            if selectedNodeID == id {
                selectedNodeID = nil
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func emptyTrash() {
        guard let p = project else { return }
        guard let trash = p.hierarchy.first(where: { $0.kind == .folder && $0.name == "Trash" }) else { return }
        var current = p
        for child in trash.children {
            do {
                current = try Writer.deleteNode(id: child.id, in: current)
            } catch {
                errorMessage = error.localizedDescription
                return
            }
        }
        project = current
    }

    func moveNodeUp(id: String) {
        guard let p = project else { return }
        guard let info = siblingIndex(of: id, in: p.hierarchy), info.index > 0 else { return }
        do {
            project = try Writer.reorderNode(id: id, newIndex: info.index - 1, in: p)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func moveNodeDown(id: String) {
        guard let p = project else { return }
        guard let info = siblingIndex(of: id, in: p.hierarchy),
              info.index < info.siblingCount - 1 else { return }
        do {
            project = try Writer.reorderNode(id: id, newIndex: info.index + 1, in: p)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func siblingIndex(of id: String, in nodes: [TreeNode]) -> (index: Int, siblingCount: Int)? {
        if let idx = nodes.firstIndex(where: { $0.id == id }) {
            return (idx, nodes.count)
        }
        for node in nodes {
            if let found = siblingIndex(of: id, in: node.children) {
                return found
            }
        }
        return nil
    }

    private func isDescendant(of folderID: String, id: String, in nodes: [TreeNode]) -> Bool {
        for node in nodes where node.kind == .folder {
            if node.id == folderID {
                return contains(id: id, in: node.children)
            }
            if isDescendant(of: folderID, id: id, in: node.children) { return true }
        }
        return false
    }

    private func contains(id: String, in nodes: [TreeNode]) -> Bool {
        for node in nodes {
            if node.id == id { return true }
            if contains(id: id, in: node.children) { return true }
        }
        return false
    }

    // MARK: - Internals

    /// Commit any uncommitted working-tree changes. Used as a safety net
    /// before destructive git operations (switch/merge/restore) so the user
    /// never loses unsaved edits.
    private func commitIfDirty(message: String, in project: Project) async {
        let url = project.path
        await Task.detached(priority: .background) {
            if (try? Git.hasChanges(in: url)) == true {
                _ = try? Git.saveRevision(message: message, in: url)
            }
        }.value
        lastAutoCommit = Date()
    }

    /// Reload the project from disk after a git operation so the editor /
    /// binder / inspector pick up the new state.
    private func reloadAfterGit(_ url: URL) async {
        let loaded = try? Reader.readProject(at: url)
        if let loaded {
            project = loaded
        } else {
            errorMessage = "Reload failed after git operation"
        }
    }

    // MARK: - Tree operations

    func createDocumentAtRoot(name: String) {
        createDocument(name: name, parentID: activeFolderParentID())
    }

    func createDocument(name: String, parentID: String?) {
        guard let p = project else { return }
        do {
            let (updated, newDoc) = try Writer.createDocument(name: name, parentID: parentID, in: p)
            project = updated
            selectedNodeID = newDoc.id
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func renameNode(id: String, newName: String) {
        guard let p = project else { return }
        do {
            project = try Writer.renameNode(id: id, newName: newName, in: p)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    /// Choose a sensible parent for a new document when the selection is on
    /// a document (the document's parent folder) or on a folder (the folder
    /// itself). Falls back to "manuscript".
    private func activeFolderParentID() -> String? {
        guard let project, let selectedID = selectedNodeID else { return "manuscript" }
        if let parent = findParent(of: selectedID, in: project.hierarchy, currentParent: nil) {
            return parent
        }
        // Selection is top-level — if it's a folder, use it; otherwise manuscript.
        if project.hierarchy.contains(where: { $0.id == selectedID && $0.kind == .folder }) {
            return selectedID
        }
        return "manuscript"
    }

    private func findParent(of id: String, in nodes: [TreeNode], currentParent: String?) -> String? {
        for node in nodes {
            if node.id == id { return currentParent }
            if let found = findParent(of: id, in: node.children, currentParent: node.id) {
                return found
            }
        }
        return nil
    }

    // MARK: - Revisions

    /// Save a named revision (git commit) with the user's message.
    func saveRevision(message: String) async {
        guard let p = project else { return }
        let trimmed = message.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        await commit(message: trimmed, in: p)
    }

    private func maybeAutoCommit() {
        guard let p = project else { return }
        let due: Bool
        if let last = lastAutoCommit {
            due = Date().timeIntervalSince(last) >= autoCommitInterval
        } else {
            due = true
        }
        guard due else { return }
        let stamp = ISO8601DateFormatter().string(from: Date())
        Task { await commit(message: "Auto: \(stamp)", in: p) }
    }

    private func commit(message: String, in project: Project) async {
        let url = project.path
        await Task.detached(priority: .background) {
            do {
                try Git.saveRevision(message: message, in: url) != nil ? () : ()
            } catch {
                await MainActor.run { [weak self] in
                    self?.errorMessage = "Commit failed: \(error.localizedDescription)"
                }
            }
        }.value
        lastAutoCommit = Date()
    }

    // MARK: - Selection helpers

    private func firstDocumentID(in nodes: [TreeNode]) -> String? {
        for node in nodes {
            if node.kind == .document { return node.id }
            if let found = firstDocumentID(in: node.children) { return found }
        }
        return nil
    }
}
