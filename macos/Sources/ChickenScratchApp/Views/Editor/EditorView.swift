import ChiknKit
import SwiftUI

struct EditorView: View {
    @Environment(ProjectStore.self) private var store

    var body: some View {
        if let doc = store.activeDocument() {
            DocumentEditor(document: doc)
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
    let document: Document
    @State private var draft: String = ""

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text(document.name)
                    .font(.system(size: 28, weight: .semibold, design: .serif))
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
        .onChange(of: document.id) { _, _ in draft = document.content }
    }
}
