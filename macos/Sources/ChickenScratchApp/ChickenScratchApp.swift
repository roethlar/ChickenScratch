import SwiftUI

@main
struct ChickenScratchApp: App {
    @State private var store = ProjectStore()
    @State private var showRevisionPrompt = false

    var body: some Scene {
        WindowGroup("ChickenScratch") {
            RootView()
                .environment(store)
                .frame(minWidth: 900, minHeight: 600)
                .sheet(isPresented: $showRevisionPrompt) {
                    RevisionPromptSheet(
                        onCommit: { message in
                            showRevisionPrompt = false
                            Task { await store.saveRevision(message: message) }
                        },
                        onCancel: { showRevisionPrompt = false }
                    )
                }
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Document") {
                    store.createDocumentAtRoot(name: "Untitled")
                }
                .keyboardShortcut("n")
                .disabled(store.project == nil)

                Divider()

                Button("Open Project…") { store.openPickedProject() }
                    .keyboardShortcut("o")
                Button("Close Project") { store.closeProject() }
                    .keyboardShortcut("w", modifiers: [.command, .shift])
                    .disabled(store.project == nil)
            }
            CommandMenu("Revision") {
                Button("Save Revision…") { showRevisionPrompt = true }
                    .keyboardShortcut("r", modifiers: .command)
                    .disabled(store.project == nil)
            }
        }
    }
}

private struct RevisionPromptSheet: View {
    let onCommit: (String) -> Void
    let onCancel: () -> Void
    @State private var message: String = ""
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Save Revision").font(.headline)
            Text("Describe this revision — e.g. \"Finished Chapter 3 rewrite\".")
                .font(.caption)
                .foregroundStyle(.secondary)
            TextField("Revision message", text: $message)
                .textFieldStyle(.roundedBorder)
                .focused($focused)
                .onSubmit(submit)
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button("Save Revision", action: submit)
                    .keyboardShortcut(.defaultAction)
                    .disabled(message.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(20)
        .frame(width: 420)
        .onAppear { focused = true }
    }

    private func submit() {
        let trimmed = message.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        onCommit(trimmed)
    }
}
