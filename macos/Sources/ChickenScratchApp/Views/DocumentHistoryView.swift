import ChiknKit
import SwiftUI

/// Per-document history modal. Lists commits that touch the given file
/// (newest first) and offers a Restore button per entry. Restore writes the
/// historical blob back and creates a new commit recording the restore —
/// forward-only, never rewrites history.
struct DocumentHistoryView: View {
    let documentID: String
    let onClose: () -> Void

    @Environment(ProjectStore.self) private var store
    @State private var revisions: [Git.RevisionEntry] = []
    @State private var isLoading = false
    @State private var busyCommit: String?
    @State private var errorMsg: String?

    private var document: Document? {
        store.project?.documents[documentID]
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("File History").font(.headline)
                    if let doc = document {
                        Text(doc.name).font(.caption).foregroundStyle(.secondary)
                    }
                }
                Spacer()
                Button {
                    onClose()
                } label: {
                    Image(systemName: "xmark.circle.fill").foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding(16)

            Divider()

            if isLoading && revisions.isEmpty {
                ProgressView().padding()
                Spacer()
            } else if revisions.isEmpty {
                VStack(spacing: 8) {
                    Image(systemName: "clock").font(.largeTitle).foregroundStyle(.secondary)
                    Text("No commits touch this file yet.").font(.body)
                    Text("Save a revision after editing to create one.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ScrollView {
                    VStack(spacing: 0) {
                        ForEach(revisions) { rev in
                            historyRow(rev)
                            Divider()
                        }
                    }
                }
            }

            if let msg = errorMsg {
                Text(msg).font(.caption).foregroundStyle(.red).padding(.horizontal, 16).padding(.vertical, 8)
            }
        }
        .frame(minWidth: 460, minHeight: 380)
        .onAppear { reload() }
    }

    @ViewBuilder
    private func historyRow(_ rev: Git.RevisionEntry) -> some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 4) {
                Text(rev.message).font(.body)
                HStack(spacing: 10) {
                    Text(rev.shortId).font(.system(.caption, design: .monospaced)).foregroundStyle(.secondary)
                    Text(rev.date.formatted(.dateTime.month(.abbreviated).day().hour().minute()))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            Spacer()
            Button {
                restore(rev)
            } label: {
                if busyCommit == rev.id {
                    ProgressView().scaleEffect(0.6).frame(width: 60)
                } else {
                    Label("Restore", systemImage: "arrow.uturn.backward").labelStyle(.titleAndIcon)
                }
            }
            .buttonStyle(.borderless)
            .disabled(busyCommit != nil)
        }
        .padding(12)
    }

    private func reload() {
        guard let doc = document, let url = store.project?.path else { return }
        isLoading = true
        let path = doc.relativePath
        Task.detached(priority: .background) {
            let history = (try? Git.documentHistory(documentPath: path, in: url)) ?? []
            await MainActor.run {
                revisions = history
                isLoading = false
            }
        }
    }

    private func restore(_ rev: Git.RevisionEntry) {
        guard let doc = document else { return }
        let path = doc.relativePath
        busyCommit = rev.id
        errorMsg = nil
        Task {
            await store.restoreDocument(documentPath: path, commit: rev.id)
            busyCommit = nil
            if let err = store.errorMessage {
                errorMsg = err
                store.errorMessage = nil
            } else {
                onClose()
            }
        }
    }
}
