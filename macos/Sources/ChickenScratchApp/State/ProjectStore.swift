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
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func closeProject() {
        project = nil
        selectedNodeID = nil
        saveState = .saved
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
