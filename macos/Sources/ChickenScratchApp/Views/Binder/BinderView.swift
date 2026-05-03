import ChiknKit
import SwiftUI

struct BinderView: View {
    @Environment(ProjectStore.self) private var store
    @State private var renameTarget: TreeNode?
    @State private var newDocParent: NewDocPrompt?
    @State private var newEntityKind: EntityKind?
    @State private var historyForDocID: String?

    var body: some View {
        @Bindable var bindableStore = store
        let hierarchy = store.project?.hierarchy ?? []
        let characters = store.entities(of: .character)
        let locations = store.entities(of: .location)
        let threadIndex = threadIndex(in: store.project)

        List(selection: $bindableStore.selectedNodeID) {
            OutlineGroup(hierarchy, id: \.id, children: \.optionalChildren) { node in
                row(for: node, threadIndex: threadIndex)
                    .tag(node.id as TreeNode.ID?)
                    .contextMenu { contextMenu(for: node) }
            }

            entitySection("Characters", kind: .character, entities: characters, threadIndex: threadIndex)
            entitySection("Locations", kind: .location, entities: locations, threadIndex: threadIndex)
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
        .sheet(item: Binding(
            get: { historyForDocID.map { HistoryTarget(documentID: $0) } },
            set: { historyForDocID = $0?.documentID }
        )) { target in
            DocumentHistoryView(documentID: target.documentID) {
                historyForDocID = nil
            }
        }
        .sheet(item: $newEntityKind) { kind in
            RenameSheet(
                initial: "",
                title: "New \(kind == .character ? "Character" : "Location")",
                placeholder: kind == .character ? "Sarah Bennett" : "Motel Room 12",
                commitLabel: "Create",
                onCommit: { name in
                    if let doc = store.createEntity(kind: kind, name: name) {
                        store.selectedNodeID = doc.id
                    }
                    newEntityKind = nil
                },
                onCancel: { newEntityKind = nil }
            )
        }
    }

    // MARK: - Rows

    @ViewBuilder
    private func row(for node: TreeNode, threadIndex: [String: ChiknKit.Thread]) -> some View {
        HStack(spacing: 6) {
            Image(systemName: icon(for: node))
                .frame(width: 16)
                .foregroundStyle(.secondary)
            Text(node.name)
            Spacer(minLength: 4)
            if node.kind == .document {
                threadDots(for: node.id, index: threadIndex)
            }
        }
    }

    @ViewBuilder
    private func entityRow(_ doc: Document, kind: EntityKind, threadIndex: [String: ChiknKit.Thread]) -> some View {
        HStack(spacing: 6) {
            Image(systemName: kind == .character ? "person.fill" : "mappin.circle.fill")
                .frame(width: 16)
                .foregroundStyle(.secondary)
            Text(doc.name)
            Spacer(minLength: 4)
            threadDots(for: doc.id, index: threadIndex)
        }
    }

    @ViewBuilder
    private func threadDots(for docID: String, index: [String: ChiknKit.Thread]) -> some View {
        if let project = store.project,
           let doc = project.documents[docID],
           let ids = doc.meta.fields["threads"]?.asStringArray,
           !ids.isEmpty {
            HStack(spacing: 2) {
                ForEach(ids.prefix(4), id: \.self) { id in
                    if let nsColor = index[id]?.color.flatMap(colorFromHex(_:)) {
                        Circle().fill(Color(nsColor: nsColor)).frame(width: 6, height: 6)
                    } else {
                        Circle().fill(Color.secondary).frame(width: 6, height: 6)
                    }
                }
            }
        }
    }

    // MARK: - Entity sections

    @ViewBuilder
    private func entitySection(
        _ title: String,
        kind: EntityKind,
        entities: [Document],
        threadIndex: [String: ChiknKit.Thread]
    ) -> some View {
        Section {
            ForEach(entities) { doc in
                entityRow(doc, kind: kind, threadIndex: threadIndex)
                    .tag(doc.id as TreeNode.ID?)
                    .contextMenu {
                        Button("Rename…") {
                            renameTarget = TreeNode(id: doc.id, name: doc.name, kind: .document)
                        }
                        Button("File History…") {
                            historyForDocID = doc.id
                        }
                    }
            }
            Button {
                newEntityKind = kind
            } label: {
                Label("New \(kind == .character ? "Character" : "Location")", systemImage: "plus")
                    .font(.caption)
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
        } header: {
            HStack {
                Text(title)
                Spacer()
                if !entities.isEmpty {
                    Text("\(entities.count)").foregroundStyle(.tertiary).font(.caption)
                }
            }
        }
    }

    // MARK: - Context menus / icons

    @ViewBuilder
    private func contextMenu(for node: TreeNode) -> some View {
        if node.kind == .folder {
            Button("New Document in \(node.name)") {
                newDocParent = NewDocPrompt(parentID: node.id)
            }
        }
        Button("Rename…") {
            renameTarget = node
        }
        if node.kind == .document {
            Button("File History…") {
                historyForDocID = node.id
            }
        }
        Divider()
        Button("Move Up") {
            store.moveNodeUp(id: node.id)
        }
        Button("Move Down") {
            store.moveNodeDown(id: node.id)
        }
        Divider()
        if isSpecialFolder(node) {
            // Manuscript / Research / Trash are structural — don't expose
            // delete on them; the user would lose the whole binder slice.
            EmptyView()
        } else if node.kind == .folder, node.name == "Trash" {
            Button("Empty Trash", role: .destructive) {
                store.emptyTrash()
            }
        } else {
            Button(role: .destructive) {
                store.deleteNode(id: node.id)
            } label: {
                Text(isInTrash(node.id) ? "Delete Permanently" : "Move to Trash")
            }
        }
    }

    private func isSpecialFolder(_ node: TreeNode) -> Bool {
        node.kind == .folder && ["Manuscript", "Research", "Templates"].contains(node.name)
    }

    private func isInTrash(_ id: String) -> Bool {
        guard let project = store.project,
              let trash = project.hierarchy.first(where: { $0.kind == .folder && $0.name == "Trash" })
        else { return false }
        return contains(id: id, in: trash.children)
    }

    private func contains(id: String, in nodes: [TreeNode]) -> Bool {
        for node in nodes {
            if node.id == id { return true }
            if contains(id: id, in: node.children) { return true }
        }
        return false
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

    private func threadIndex(in project: Project?) -> [String: ChiknKit.Thread] {
        guard let project else { return [:] }
        return Dictionary(uniqueKeysWithValues: project.threads.map { ($0.id, $0) })
    }
}

private struct NewDocPrompt: Identifiable {
    let id = UUID()
    let parentID: String?
}

private struct HistoryTarget: Identifiable {
    let documentID: String
    var id: String { documentID }
}

extension EntityKind: Identifiable {
    public var id: String { rawValue }
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
