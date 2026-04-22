import ChiknKit
import SwiftUI

struct BinderView: View {
    @Environment(ProjectStore.self) private var store

    var body: some View {
        @Bindable var bindableStore = store
        let hierarchy = store.project?.hierarchy ?? []

        List(hierarchy, children: \.optionalChildren, selection: $bindableStore.selectedNodeID) { node in
            Label(node.name, systemImage: icon(for: node))
                .tag(node.id as TreeNode.ID?)
        }
        .listStyle(.sidebar)
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

private extension TreeNode {
    /// `List(children:)` uses `nil` to mean "leaf"; empty arrays render a disclosure chevron.
    var optionalChildren: [TreeNode]? {
        children.isEmpty ? nil : children
    }
}
