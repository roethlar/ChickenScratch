import ChiknKit
import SwiftUI

struct EditorView: View {
    @Environment(ProjectStore.self) private var store

    var body: some View {
        if let doc = store.activeDocument() {
            DocumentEditor(document: doc)
                .id(doc.id)
        } else {
            ContentUnavailableView(
                "Select a document",
                systemImage: "doc.text",
                description: Text("Pick a scene or chapter from the binder.")
            )
        }
    }
}

private struct DocumentEditor: View {
    @Environment(ProjectStore.self) private var store
    let document: Document

    @State private var draft: String = ""
    @State private var pendingSave: Task<Void, Never>?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                HStack {
                    Text(document.name)
                        .font(.system(size: 28, weight: .semibold, design: .serif))
                    Spacer()
                    SaveIndicator()
                }
                .padding(.top, 32)

                TextEditor(text: $draft)
                    .font(.system(size: 16, design: .serif))
                    .lineSpacing(6)
                    .scrollContentBackground(.hidden)
                    .frame(minHeight: 400)
            }
            .frame(maxWidth: 720)
            .padding(.horizontal, 48)
            .padding(.bottom, 32)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .onAppear { draft = document.content }
        .onChange(of: draft) { _, new in scheduleSave(new) }
        .onDisappear { flushSave() }
    }

    private func scheduleSave(_ content: String) {
        // Mark dirty immediately so the indicator reflects the keystroke.
        if content != document.content {
            store.saveState = .dirty
        }

        pendingSave?.cancel()
        let id = document.id
        pendingSave = Task { @MainActor in
            try? await Task.sleep(for: .seconds(1.2))
            if Task.isCancelled { return }
            store.saveDocument(id: id, content: content)
        }
    }

    private func flushSave() {
        pendingSave?.cancel()
        if draft != document.content {
            store.saveDocument(id: document.id, content: draft)
        }
    }
}

private struct SaveIndicator: View {
    @Environment(ProjectStore.self) private var store

    var body: some View {
        switch store.saveState {
        case .saved:
            Label("Saved", systemImage: "checkmark.circle")
                .foregroundStyle(.secondary)
                .font(.caption)
        case .dirty:
            Label("Modified", systemImage: "circle.dotted")
                .foregroundStyle(.secondary)
                .font(.caption)
        case .saving:
            Label("Saving…", systemImage: "arrow.triangle.2.circlepath")
                .foregroundStyle(.secondary)
                .font(.caption)
        case .failed(let msg):
            Label(msg, systemImage: "exclamationmark.triangle")
                .foregroundStyle(.red)
                .font(.caption)
        }
    }
}
