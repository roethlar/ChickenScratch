import ChiknKit
import SwiftUI

struct RevisionsView: View {
    @Environment(ProjectStore.self) private var store
    @State private var revisions: [Git.RevisionEntry] = []
    @State private var selected: Git.RevisionEntry?
    @State private var isLoading = false
    @State private var errorMsg: String?

    var body: some View {
        VStack(spacing: 0) {
            List(revisions, selection: $selected) { rev in
                VStack(alignment: .leading, spacing: 3) {
                    Text(rev.message).font(.body)
                    HStack(spacing: 10) {
                        Text(rev.shortId).font(.system(.caption, design: .monospaced)).foregroundStyle(.secondary)
                        Text(rev.date, style: .date).font(.caption).foregroundStyle(.secondary)
                        Text(rev.date, style: .time).font(.caption).foregroundStyle(.secondary)
                    }
                }
                .padding(.vertical, 2)
                .tag(rev as Git.RevisionEntry?)
            }
            .listStyle(.sidebar)

            Divider()

            HStack {
                if let msg = errorMsg {
                    Text(msg).font(.caption).foregroundStyle(.red).lineLimit(1)
                }
                Spacer()
                if isLoading { ProgressView().scaleEffect(0.7) }
                Button("Restore") {
                    guard let rev = selected else { return }
                    restore(rev)
                }
                .disabled(selected == nil || isLoading)
                .buttonStyle(.glass)
            }
            .padding(10)
        }
        .onAppear { loadRevisions() }
        .onChange(of: store.project?.id) { _, _ in loadRevisions() }
        .navigationTitle("Revisions")
    }

    private func loadRevisions() {
        guard let url = store.project?.path else { revisions = []; return }
        isLoading = true
        Task.detached(priority: .background) {
            let result = try? Git.listRevisions(in: url)
            await MainActor.run {
                revisions = result ?? []
                isLoading = false
            }
        }
    }

    private func restore(_ rev: Git.RevisionEntry) {
        guard let url = store.project?.path else { return }
        isLoading = true
        errorMsg = nil
        Task.detached(priority: .background) {
            do {
                try Git.restoreRevision(commitHash: rev.id, in: url)
                await MainActor.run {
                    isLoading = false
                    store.open(url: url)
                }
            } catch {
                await MainActor.run {
                    errorMsg = error.localizedDescription
                    isLoading = false
                }
            }
        }
    }
}
