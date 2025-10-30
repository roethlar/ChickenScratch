import SwiftUI

struct InspectorView: View {
    @EnvironmentObject private var projectViewModel: ProjectViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Inspector")
                .font(.headline)
            Divider()

            if let document = projectViewModel.selectedDocument {
                Form {
                    Section("Details") {
                        TextField("Title", text: Binding(
                            get: { document.metadata.name },
                            set: { updateName($0) }
                        ))
                        Text("Word Count: \(document.content.split(separator: " ").count)")
                            .foregroundStyle(.secondary)
                    }

                    if let label = document.metadata.label {
                        Section("Label") {
                            Text(label)
                        }
                    }

                    if let status = document.metadata.status {
                        Section("Status") {
                            Text(status)
                        }
                    }

                    if let keywords = document.metadata.keywords, !keywords.isEmpty {
                        Section("Keywords") {
                            Text(keywords.joined(separator: ", "))
                        }
                    }

                    if let synopsis = document.metadata.synopsis {
                        Section("Synopsis") {
                            Text(synopsis)
                                .font(.callout)
                                .foregroundStyle(.secondary)
                        }
                    }
                }
            } else {
                Text("Select a document to view details.")
                    .foregroundStyle(.secondary)
            }
            Spacer()
        }
        .padding()
        .frame(minWidth: 220, idealWidth: 260)
    }

    private func updateName(_ name: String) {
        guard let id = projectViewModel.selectedDocumentID else { return }
        projectViewModel.renameDocument(id: id, to: name)
    }
}
