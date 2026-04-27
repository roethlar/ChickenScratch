import SwiftUI

struct ProjectWindow: View {
    @Environment(ProjectStore.self) private var store
    @State private var columnVisibility: NavigationSplitViewVisibility = .all
    @State private var showRevisions = false
    @State private var showSaveRevision = false
    @State private var revisionMessage = ""

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

                    ToolbarItem(placement: .secondaryAction) {
                        Button {
                            showRevisions.toggle()
                        } label: {
                            Label("Revisions", systemImage: "clock.arrow.circlepath")
                        }
                        .buttonStyle(.glass)
                        .keyboardShortcut("r", modifiers: [.command, .shift])
                    }

                    ToolbarItem(placement: .secondaryAction) {
                        Button {
                            showSaveRevision.toggle()
                        } label: {
                            Label("Save Revision", systemImage: "square.and.arrow.down")
                        }
                        .buttonStyle(.glass)
                        .keyboardShortcut("s", modifiers: [.command, .shift])
                    }
                }
        }
        .sheet(isPresented: $showRevisions) {
            RevisionsView()
                .environment(store)
                .frame(minWidth: 420, minHeight: 480)
        }
        .sheet(isPresented: $showSaveRevision) {
            SaveRevisionSheet(
                message: $revisionMessage,
                onCommit: {
                    let msg = revisionMessage
                    showSaveRevision = false
                    revisionMessage = ""
                    Task { await store.saveRevision(message: msg) }
                },
                onCancel: {
                    showSaveRevision = false
                    revisionMessage = ""
                }
            )
        }
    }
}

private struct SaveRevisionSheet: View {
    @Binding var message: String
    let onCommit: () -> Void
    let onCancel: () -> Void
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Save Revision").font(.headline)

            TextField("Revision message", text: $message)
                .textFieldStyle(.roundedBorder)
                .focused($focused)

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button("Save") { onCommit() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(message.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(24)
        .frame(width: 360)
        .onAppear { focused = true }
    }
}
