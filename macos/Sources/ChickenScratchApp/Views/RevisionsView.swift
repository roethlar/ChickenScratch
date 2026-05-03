import ChiknKit
import SwiftUI

struct RevisionsView: View {
    @Environment(ProjectStore.self) private var store
    @State private var tab: Tab = .history
    @State private var revisions: [Git.RevisionEntry] = []
    @State private var drafts: [Git.DraftVersion] = []
    @State private var dangling: [DanglingRef] = []
    @State private var selectedRevision: Git.RevisionEntry?
    @State private var isLoading = false
    @State private var errorMsg: String?
    @State private var newDraftSheet = false

    enum Tab: String, CaseIterable, Identifiable {
        case history = "History"
        case drafts = "Drafts"
        case threads = "Threads"
        var id: String { rawValue }
    }

    var body: some View {
        VStack(spacing: 0) {
            Picker("", selection: $tab) {
                ForEach(Tab.allCases) { t in Text(t.rawValue).tag(t) }
            }
            .pickerStyle(.segmented)
            .padding(10)

            Divider()

            Group {
                switch tab {
                case .history: historyTab
                case .drafts: draftsTab
                case .threads: threadsTab
                }
            }

            Divider()

            HStack {
                if let msg = errorMsg {
                    Text(msg).font(.caption).foregroundStyle(.red).lineLimit(2)
                }
                Spacer()
                if isLoading { ProgressView().scaleEffect(0.7) }
                tabFooterButton
            }
            .padding(10)
        }
        .navigationTitle("Revisions")
        .onAppear { reload() }
        .onChange(of: store.project?.id) { _, _ in reload() }
        .onChange(of: tab) { _, _ in reload() }
        .sheet(isPresented: $newDraftSheet) {
            NewDraftSheet(
                onCommit: { name in
                    newDraftSheet = false
                    Task { await store.createDraft(name: name); reload() }
                },
                onCancel: { newDraftSheet = false }
            )
        }
    }

    // MARK: - History tab

    @ViewBuilder
    private var historyTab: some View {
        List(revisions, selection: $selectedRevision) { rev in
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
    }

    // MARK: - Drafts tab

    @ViewBuilder
    private var draftsTab: some View {
        List {
            ForEach(drafts) { draft in
                HStack {
                    Image(systemName: "arrow.triangle.branch")
                        .foregroundStyle(draft.isActive ? Color.accentColor : .secondary)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(draft.name).font(.body)
                        if draft.isActive {
                            Text("Active").font(.caption).foregroundStyle(.secondary)
                        }
                    }
                    Spacer()
                    if !draft.isActive {
                        Button("Switch") {
                            Task { await store.switchDraft(name: draft.name); reload() }
                        }
                        .buttonStyle(.borderless)
                        Button("Merge") {
                            Task { await store.mergeDraft(name: draft.name); reload() }
                        }
                        .buttonStyle(.borderless)
                    }
                }
                .padding(.vertical, 4)
            }
            if drafts.isEmpty {
                Text("No drafts yet — create one to branch off the current state.")
                    .foregroundStyle(.secondary)
                    .padding()
            }
        }
        .listStyle(.sidebar)
    }

    // MARK: - Threads tab

    @ViewBuilder
    private var threadsTab: some View {
        let project = store.project
        let threads = project?.threads ?? []
        let scenesByThread = scenesByThreadID(in: project)

        VStack(spacing: 0) {
            if !dangling.isEmpty {
                DanglingRefsBanner(dangling: dangling)
            }
            List {
                ForEach(threads) { thread in
                    DisclosureGroup {
                        let scenes = scenesByThread[thread.id] ?? []
                        if scenes.isEmpty {
                            Text("No scenes tagged.").foregroundStyle(.secondary).font(.caption)
                        } else {
                            ForEach(scenes, id: \.id) { doc in
                                Button {
                                    store.selectedNodeID = doc.id
                                } label: {
                                    HStack {
                                        Image(systemName: "doc.text").foregroundStyle(.secondary)
                                        Text(doc.name)
                                        Spacer()
                                    }
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    } label: {
                        HStack {
                            if let nsColor = thread.color.flatMap(colorFromHex) {
                                Circle().fill(Color(nsColor: nsColor)).frame(width: 10, height: 10)
                            } else {
                                Circle().fill(Color.secondary).frame(width: 10, height: 10)
                            }
                            Text(thread.name)
                            Spacer()
                            Text("\((scenesByThread[thread.id] ?? []).count) scene\((scenesByThread[thread.id] ?? []).count == 1 ? "" : "s")")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Button {
                                store.deleteThread(id: thread.id)
                                reload()
                            } label: {
                                Image(systemName: "trash").foregroundStyle(.red.opacity(0.7))
                            }
                            .buttonStyle(.borderless)
                        }
                    }
                }
                if threads.isEmpty {
                    Text("No plot threads yet. Tag scenes via the Inspector \"Threads\" field, or create one here.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .padding()
                }
            }
            .listStyle(.sidebar)
        }
    }

    // MARK: - Footer button per tab

    @ViewBuilder
    private var tabFooterButton: some View {
        switch tab {
        case .history:
            Button("Restore") {
                guard let rev = selectedRevision else { return }
                restore(rev)
            }
            .disabled(selectedRevision == nil || isLoading)
            .buttonStyle(.glass)
        case .drafts:
            Button {
                newDraftSheet = true
            } label: {
                Label("New Draft", systemImage: "plus")
            }
            .buttonStyle(.glass)
        case .threads:
            Button {
                Task { @MainActor in
                    store.createThread(name: "Untitled thread")
                    reload()
                }
            } label: {
                Label("New Thread", systemImage: "plus")
            }
            .buttonStyle(.glass)
        }
    }

    // MARK: - Reload / actions

    private func reload() {
        guard let url = store.project?.path else {
            revisions = []
            drafts = []
            dangling = []
            return
        }
        isLoading = true
        let project = store.project
        Task.detached(priority: .background) {
            let revs = (try? Git.listRevisions(in: url)) ?? []
            let drs  = (try? Git.listDrafts(in: url)) ?? []
            let dang = project.map(References.validate) ?? []
            await MainActor.run {
                revisions = revs
                drafts = drs
                dangling = dang
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

    private func scenesByThreadID(in project: Project?) -> [String: [Document]] {
        guard let project else { return [:] }
        var out: [String: [Document]] = [:]
        for doc in project.documents.values {
            guard let ids = doc.meta.fields["threads"]?.asStringArray else { continue }
            for id in ids {
                out[id, default: []].append(doc)
            }
        }
        for key in out.keys {
            out[key]?.sort { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        }
        return out
    }
}

private struct DanglingRefsBanner: View {
    let dangling: [DanglingRef]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(.orange)
                Text("\(dangling.count) dangling reference\(dangling.count == 1 ? "" : "s")")
                    .font(.caption.weight(.semibold))
                Spacer()
            }
            Text("Scenes pointing at deleted characters / locations / threads. Open the scene's Inspector to clear them.")
                .font(.caption)
                .foregroundStyle(.secondary)
            DisclosureGroup("Show") {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(dangling.prefix(20)) { ref in
                        HStack(spacing: 6) {
                            Text(ref.documentName).font(.caption.italic())
                            Text("·").foregroundStyle(.secondary)
                            Text(ref.field).font(.caption).foregroundStyle(.secondary)
                            Text(ref.missingID).font(.system(.caption, design: .monospaced))
                        }
                    }
                    if dangling.count > 20 {
                        Text("…and \(dangling.count - 20) more")
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    }
                }
            }
            .font(.caption)
        }
        .padding(10)
        .background(.orange.opacity(0.08))
    }
}

private struct NewDraftSheet: View {
    let onCommit: (String) -> Void
    let onCancel: () -> Void
    @State private var name = ""
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("New Draft").font(.headline)
            Text("Branches off the current draft. Switch back any time — both drafts keep their own history.")
                .font(.caption)
                .foregroundStyle(.secondary)
            TextField("Draft name", text: $name)
                .textFieldStyle(.roundedBorder)
                .focused($focused)
                .onSubmit {
                    let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !trimmed.isEmpty { onCommit(trimmed) }
                }
            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { onCancel() }
                    .keyboardShortcut(.cancelAction)
                Button("Create") {
                    let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !trimmed.isEmpty { onCommit(trimmed) }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(20)
        .frame(width: 380)
        .onAppear { focused = true }
    }
}
