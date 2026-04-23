import ChiknKit
import SwiftUI

struct BinderView: View {
    @Environment(ProjectStore.self) private var store
    @State private var renameTarget: TreeNode?
    @State private var renameText: String = ""
    @State private var newDocParent: NewDocPrompt?

    var body: some View {
        @Bindable var bindableStore = store
        let hierarchy = store.project?.hierarchy ?? []

        List(
            hierarchy,
            children: \.optionalChildren,
            selection: $bindableStore.selectedNodeID
        ) { node in
            Label(node.name, systemImage: icon(for: node))
                .tag(node.id as TreeNode.ID?)
                .contextMenu { contextMenu(for: node) }
        }
        .listStyle(.sidebar)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    newDocParent = NewDocPrompt(parentID: nil)
                } label: {
                    Label("New Document", systemImage: "plus")
                }
                .buttonStyle(.glass)
                .help("New Document (⌘N)")
            }
        }
        .sheet(item: $renameTarget) { target in
            RenameSheet(
                initial: target.name,
                title: "Rename \(target.name)",
                onCommit: { newName in
                    store.renameNode(id: target.id, newName: newName)
                    renameTarget = nil
                },
                onCancel: { renameTarget = nil }
            )
        }
        .sheet(item: $newDocParent) { prompt in
            RenameSheet(
                initial: "",
                title: "New Document",
                placeholder: "Scene name",
                commitLabel: "Create",
                onCommit: { name in
                    store.createDocument(name: name, parentID: prompt.parentID)
                    newDocParent = nil
                },
                onCancel: { newDocParent = nil }
            )
        }
    }

    @ViewBuilder
    private func contextMenu(for node: TreeNode) -> some View {
        if node.kind == .folder {
            Button("New Document in \(node.name)") {
                newDocParent = NewDocPrompt(parentID: node.id)
            }
        }
        Button("Rename…") {
            renameText = node.name
            renameTarget = node
        }
    }

    private func icon(for node: TreeNode) -> String {
        switch node.kind {
        case .folder:
            switch node.id {
            case "manuscript": "books.vertical"
            case "research": "tray.full"
            case "trash": "trash"
            default: "folder"
            }
        case .document:
            "doc.text"
        }
    }
}

private struct NewDocPrompt: Identifiable {
    let id = UUID()
    let parentID: String?
}

private struct RenameSheet: View {
    let initial: String
    let title: String
    var placeholder: String = "Name"
    var commitLabel: String = "Rename"
    let onCommit: (String) -> Void
    let onCancel: () -> Void

    @State private var text: String = ""
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title).font(.headline)
            TextField(placeholder, text: $text)
                .textFieldStyle(.roundedBorder)
                .focused($focused)
                .onSubmit(commit)
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button(commitLabel, action: commit)
                    .keyboardShortcut(.defaultAction)
                    .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(20)
        .frame(width: 360)
        .onAppear {
            text = initial
            focused = true
        }
    }

    private func commit() {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        onCommit(trimmed)
    }
}

private extension TreeNode {
    var optionalChildren: [TreeNode]? {
        children.isEmpty ? nil : children
    }
}
