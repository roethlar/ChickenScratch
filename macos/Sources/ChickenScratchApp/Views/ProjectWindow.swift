import SwiftUI

struct ProjectWindow: View {
    @Environment(ProjectStore.self) private var store
    @State private var columnVisibility: NavigationSplitViewVisibility = .all

    var body: some View {
        @Bindable var bindableStore = store

        NavigationSplitView(columnVisibility: $columnVisibility) {
            BinderView()
                .navigationTitle(store.project?.name ?? "")
                .navigationSplitViewColumnWidth(min: 200, ideal: 240, max: 360)
        } detail: {
            EditorView()
                .inspector(isPresented: $bindableStore.showInspector) {
                    InspectorView()
                        .inspectorColumnWidth(min: 260, ideal: 300, max: 420)
                }
                .toolbar {
                    ToolbarItem(placement: .primaryAction) {
                        Button {
                            bindableStore.showInspector.toggle()
                        } label: {
                            Label("Inspector", systemImage: "sidebar.trailing")
                        }
                        .buttonStyle(.glass)
                        .keyboardShortcut("i", modifiers: [.command, .shift])
                    }
                }
        }
    }
}
