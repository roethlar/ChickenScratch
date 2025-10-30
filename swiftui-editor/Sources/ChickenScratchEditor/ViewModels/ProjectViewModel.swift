import Foundation
import Combine
import AppKit

@MainActor
final class ProjectViewModel: ObservableObject {
    @Published var project: ChiknProject?
    @Published var selectedDocumentID: String?
    @Published var editorContent: NSAttributedString = NSAttributedString(string: "")
    @Published var statusMessage: String?
    @Published var errorMessage: String?
    @Published var isDirty: Bool = false
    @Published var isLoading: Bool = false

    private let loader = ChiknProjectLoader()
    private let writer = ChiknProjectWriter()
    private let transformer = MarkdownTransformer()

    var projectName: String {
        project?.name ?? "Chicken Scratch"
    }

    var selectedDocument: ChiknDocument? {
        guard let id = selectedDocumentID else { return nil }
        return project?.documents[id]
    }

    func presentOpenPanel() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Open"
        panel.title = "Open Chicken Scratch Project"

        panel.begin { [weak self] response in
            guard response == .OK, let url = panel.url else { return }
            Task { await self?.openProject(at: url) }
        }
    }

    func openProject(at url: URL) async {
        isLoading = true
        defer { isLoading = false }

        do {
            let project = try loader.loadProject(at: url)
            self.project = project
            self.selectedDocumentID = firstDocumentID(in: project.hierarchy)
            if let docID = selectedDocumentID,
               let document = project.documents[docID] {
                self.editorContent = document.attributedContent(using: transformer)
            } else {
                self.editorContent = NSAttributedString(string: "")
            }
            self.statusMessage = "Opened \(project.name)"
        } catch {
            self.errorMessage = "Failed to open project: \(error.localizedDescription)"
        }
    }

    func select(node: ChiknTreeNode) {
        switch node {
        case .folder:
            return
        case .document(let docNode):
            selectedDocumentID = docNode.id
            if let document = project?.documents[docNode.id] {
                editorContent = document.attributedContent(using: transformer)
            } else {
                editorContent = NSAttributedString(string: "")
            }
        }
    }

    func updateEditorContent(_ attributedString: NSAttributedString) {
        guard var document = selectedDocument,
              var project else { return }

        let markdown = transformer.markdown(from: attributedString)
        document.content = markdown
        document.metadata = DocumentMetadata(
            projectURL: document.metadata.projectURL,
            name: document.metadata.name,
            created: document.metadata.created,
            modified: Date(),
            parentID: document.metadata.parentID,
            label: document.metadata.label,
            status: document.metadata.status,
            keywords: document.metadata.keywords,
            synopsis: document.metadata.synopsis
        )
        project.documents[document.id] = document
        self.project = project
        self.editorContent = attributedString
        self.isDirty = true
    }

    func saveCurrentDocument() {
        guard isDirty, let document = selectedDocument else { return }
        do {
            try writer.saveDocument(document)
            isDirty = false
            statusMessage = "Saved \(document.name)"
        } catch {
            errorMessage = "Failed to save document: \(error.localizedDescription)"
        }
    }

    func saveProject() {
        guard var project else { return }
        project.modified = Date()
        self.project = project
        do {
            try writer.saveProject(project)
        } catch {
            errorMessage = "Failed to update project: \(error.localizedDescription)"
        }
    }

    private func firstDocumentID(in nodes: [ChiknTreeNode]) -> String? {
        for node in nodes {
            switch node {
            case .folder(let folder):
                if let id = firstDocumentID(in: folder.children) {
                    return id
                }
            case .document(let document):
                return document.id
            }
        }
        return nil
    }

    func renameDocument(id: String, to name: String) {
        guard var project = project, var document = project.documents[id] else { return }
        document.name = name
        document.metadata = DocumentMetadata(
            projectURL: document.metadata.projectURL,
            name: name,
            created: document.metadata.created,
            modified: Date(),
            parentID: document.metadata.parentID,
            label: document.metadata.label,
            status: document.metadata.status,
            keywords: document.metadata.keywords,
            synopsis: document.metadata.synopsis
        )
        project.documents[id] = document
        project.hierarchy = updateHierarchy(project.hierarchy, documentID: id, name: name)
        self.project = project
        isDirty = true
        saveProject()
        saveCurrentDocument()
    }

    private func updateHierarchy(
        _ nodes: [ChiknTreeNode],
        documentID: String,
        name: String
    ) -> [ChiknTreeNode] {
        nodes.map { node in
            switch node {
            case .folder(let folder):
                let updatedChildren = updateHierarchy(folder.children, documentID: documentID, name: name)
                return .folder(
                    ChiknTreeNode.FolderNode(
                        id: folder.id,
                        name: folder.name,
                        children: updatedChildren
                    )
                )
            case .document(let document):
                if document.id == documentID {
                    return .document(
                        ChiknTreeNode.DocumentNode(
                            id: document.id,
                            name: name,
                            path: document.path
                        )
                    )
                }
                return node
            }
        }
    }
}
