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
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func closeProject() {
        project = nil
        selectedNodeID = nil
    }

    func activeDocument() -> Document? {
        guard let project, let id = selectedNodeID else { return nil }
        return project.documents[id]
    }

    private func firstDocumentID(in nodes: [TreeNode]) -> String? {
        for node in nodes {
            if node.kind == .document { return node.id }
            if let found = firstDocumentID(in: node.children) { return found }
        }
        return nil
    }
}
