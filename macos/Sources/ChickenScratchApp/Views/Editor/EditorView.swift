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
        ZStack(alignment: .bottomTrailing) {
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

            SessionBadge()
                .padding(.trailing, 16)
                .padding(.bottom, 16)
        }
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

/// Compact, idle-hiding overlay that surfaces session-target progress while
/// the writer types. Suppresses itself entirely when no session target is
/// configured. Mirrors the Tauri SessionBadge.
private struct SessionBadge: View {
    @Environment(ProjectStore.self) private var store
    @State private var progress: SessionProgress?
    @State private var hidden = false
    @State private var hideTask: Task<Void, Never>?

    var body: some View {
        Group {
            if let progress, hasTarget(progress), !hidden {
                badge(progress)
                    .transition(.opacity)
            } else {
                EmptyView()
            }
        }
        .onAppear { reload(); scheduleHide() }
        .onChange(of: store.activeDocument()?.id) { _, _ in reload() }
        .onChange(of: store.saveState) { _, state in
            // Treat any save activity as "user is writing" — refresh and
            // re-show the badge briefly so they can see progress update.
            if case .saving = state {
                reload()
                hidden = false
                scheduleHide()
            }
        }
    }

    @ViewBuilder
    private func badge(_ p: SessionProgress) -> some View {
        let goal = p.wordsPerSession ?? 0
        let pct = goal > 0 ? min(1.0, Double(p.todayWords) / Double(goal)) : 0
        let reached = goal > 0 && p.todayWords >= goal

        VStack(alignment: .trailing, spacing: 4) {
            Text(badgeText(p))
                .font(.caption.monospacedDigit())
                .foregroundStyle(reached ? .green : .primary)
            if goal > 0 {
                ProgressView(value: pct)
                    .progressViewStyle(.linear)
                    .tint(reached ? .green : .accentColor)
                    .frame(width: 140)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
        .help("Session target progress")
    }

    private func badgeText(_ p: SessionProgress) -> String {
        var parts: [String] = []
        if let goal = p.wordsPerSession, goal > 0 {
            parts.append("Today \(p.todayWords)/\(goal)")
        }
        if let days = p.daysRemaining {
            if days > 0 { parts.append("\(days)d left") }
            else if days == 0 { parts.append("deadline today") }
            else { parts.append("deadline passed") }
        }
        if let need = p.neededPerDay {
            parts.append("\(need)/day needed")
        }
        return parts.joined(separator: " · ")
    }

    private func hasTarget(_ p: SessionProgress) -> Bool {
        p.wordsPerSession != nil || p.totalTarget != nil || p.deadline != nil
    }

    private func reload() {
        progress = store.sessionProgress()
    }

    private func scheduleHide() {
        hideTask?.cancel()
        hideTask = Task { @MainActor in
            try? await Task.sleep(for: .seconds(4))
            if !Task.isCancelled { hidden = true }
        }
    }
}
