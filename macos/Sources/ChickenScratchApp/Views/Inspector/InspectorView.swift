import ChiknKit
import SwiftUI

struct InspectorView: View {
    @Environment(ProjectStore.self) private var store

    var body: some View {
        if let doc = store.activeDocument() {
            InspectorForm(document: doc)
        } else {
            VStack {
                Spacer()
                Text("No document selected")
                    .foregroundStyle(.secondary)
                Spacer()
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

private struct InspectorForm: View {
    let document: Document
    @Environment(ProjectStore.self) private var store

    @State private var synopsis: String = ""
    @State private var label: String = ""
    @State private var status: String = ""
    @State private var keywords: String = ""
    @State private var includeInCompile: Bool = true
    @State private var wordTarget: String = ""

    var body: some View {
        GlassEffectContainer(spacing: 12) {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    field(title: "Synopsis") {
                        TextEditor(text: $synopsis)
                            .font(.body)
                            .frame(minHeight: 72)
                            .scrollContentBackground(.hidden)
                            .background(.quaternary.opacity(0.3))
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                            .onChange(of: synopsis) { _, _ in saveMeta() }
                    }

                    field(title: "Label") {
                        TextField("e.g. Scene, Chapter", text: $label)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: label) { _, _ in saveMeta() }
                    }

                    field(title: "Status") {
                        TextField("e.g. Draft, To Do, Final", text: $status)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: status) { _, _ in saveMeta() }
                    }

                    field(title: "Keywords") {
                        TextField("comma-separated", text: $keywords)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: keywords) { _, _ in saveMeta() }
                    }

                    field(title: "Compile") {
                        Toggle("Include in compile", isOn: $includeInCompile)
                            .onChange(of: includeInCompile) { _, _ in saveMeta() }
                        TextField("Word count target", text: $wordTarget)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: wordTarget) { _, _ in saveMeta() }
                    }

                    field(title: "Word count") {
                        Text("\(wordCount(document.content))")
                            .font(.title2.weight(.medium))
                            .monospacedDigit()
                    }
                }
                .padding(20)
            }
            .panelGlass(cornerRadius: 20)
            .padding(12)
        }
        .onAppear { loadFields() }
        .onChange(of: document.id) { _, _ in loadFields() }
    }

    private func loadFields() {
        synopsis   = document.meta.synopsis ?? ""
        label      = document.meta.label ?? ""
        status     = document.meta.status ?? ""
        keywords   = document.meta.keywords.joined(separator: ", ")
        includeInCompile = document.meta.includeInCompile
        wordTarget = document.meta.wordCountTarget.map(String.init) ?? ""
    }

    private func saveMeta() {
        let kws = keywords.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        let meta = DocumentMeta(
            synopsis: synopsis.isEmpty ? nil : synopsis,
            label: label.isEmpty ? nil : label,
            status: status.isEmpty ? nil : status,
            keywords: kws,
            includeInCompile: includeInCompile,
            wordCountTarget: Int(wordTarget)
        )
        store.saveDocumentMeta(id: document.id, meta: meta)
    }

    private func field<Content: View>(title: String, @ViewBuilder _ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title.uppercased()).font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
            content()
        }
    }

    private func wordCount(_ text: String) -> Int {
        text.split { $0.isWhitespace || $0.isNewline }.count
    }
}
