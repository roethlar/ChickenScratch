import ChiknKit
import SwiftUI

/// Project-wide statistics panel — manuscript word count, pages, reading
/// time, daily history chart, per-doc breakdown, and a session-target editor.
/// Mirrors the Tauri `StatsPanel.tsx`.
struct StatsView: View {
    let onClose: () -> Void

    @Environment(ProjectStore.self) private var store
    @State private var stats: ProjectStats?
    @State private var history: [DayEntry] = []

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "chart.bar.xaxis")
                Text("Statistics").font(.headline)
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

            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    if let stats {
                        summarySection(stats)
                        SessionTargetSection()
                        if history.count > 1 {
                            historyChart
                        }
                        perDocSection(stats)
                    } else {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                            .padding(40)
                    }
                }
                .padding(16)
            }
        }
        .frame(minWidth: 480, minHeight: 520)
        .onAppear { reload() }
    }

    @ViewBuilder
    private func summarySection(_ stats: ProjectStats) -> some View {
        let pages = Int(ceil(Double(stats.manuscriptWords) / 250.0))
        let readingTime = Int(ceil(Double(stats.totalWords) / 200.0))
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .top, spacing: 24) {
                bigStat(value: stats.manuscriptWords.formatted(), label: "manuscript words")
                bigStat(value: "~\(pages)", label: "pages")
                bigStat(value: "~\(readingTime)m", label: "read time")
                Spacer()
            }
            Text("\(stats.totalDocs) documents · \(stats.totalWords.formatted()) total words")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private func bigStat(value: String, label: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(value).font(.system(size: 24, weight: .semibold, design: .serif)).monospacedDigit()
            Text(label).font(.caption).foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    private var historyChart: some View {
        let recent = Array(history.suffix(14))
        let maxWords = max(recent.map(\.words).max() ?? 1, 1)
        VStack(alignment: .leading, spacing: 6) {
            Text("DAILY WORD COUNT")
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
            HStack(alignment: .bottom, spacing: 4) {
                ForEach(recent, id: \.date) { day in
                    let h = max(2, CGFloat(day.words) / CGFloat(maxWords) * 100)
                    VStack(spacing: 4) {
                        Spacer(minLength: 0)
                        RoundedRectangle(cornerRadius: 2)
                            .fill(Color.accentColor.opacity(0.7))
                            .frame(height: h)
                        Text(String(day.date.suffix(5))) // MM-DD
                            .font(.system(size: 9, design: .monospaced))
                            .foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity)
                    .help("\(day.date): \(day.words.formatted()) words")
                }
            }
            .frame(height: 130)
        }
    }

    @ViewBuilder
    private func perDocSection(_ stats: ProjectStats) -> some View {
        let maxWords = max(stats.docs.map(\.words).max() ?? 1, 1)
        VStack(alignment: .leading, spacing: 8) {
            Text("PER DOCUMENT").font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
            ForEach(stats.docs) { doc in
                Button {
                    store.selectedNodeID = doc.id
                    onClose()
                } label: {
                    VStack(alignment: .leading, spacing: 2) {
                        HStack {
                            Image(systemName: doc.includeInCompile ? "book" : "doc.text")
                                .foregroundStyle(.secondary)
                            Text(doc.name)
                            Spacer()
                            Text(doc.words.formatted())
                                .font(.system(.caption, design: .monospaced))
                                .foregroundStyle(.secondary)
                        }
                        GeometryReader { geo in
                            ZStack(alignment: .leading) {
                                RoundedRectangle(cornerRadius: 2)
                                    .fill(.quaternary.opacity(0.4))
                                    .frame(height: 4)
                                RoundedRectangle(cornerRadius: 2)
                                    .fill(Color.accentColor.opacity(0.6))
                                    .frame(width: geo.size.width * CGFloat(doc.words) / CGFloat(maxWords), height: 4)
                            }
                        }
                        .frame(height: 4)
                    }
                    .padding(.vertical, 4)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func reload() {
        stats = store.projectStats()
        store.recordDailyWordsNow()
        history = store.writingHistory().entries
    }
}

private struct SessionTargetSection: View {
    @Environment(ProjectStore.self) private var store
    @State private var progress: SessionProgress?
    @State private var editing = false
    @State private var wordsPerSession = ""
    @State private var deadline = ""
    @State private var totalTarget = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("SESSION TARGET").font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
                Spacer()
                Button(editing ? "Cancel" : (hasTarget ? "Edit" : "Configure")) {
                    if editing {
                        loadFromProgress()
                    }
                    editing.toggle()
                }
                .buttonStyle(.borderless)
                .font(.caption)
            }

            if !editing {
                if let progress, hasTarget {
                    displayRows(progress)
                } else {
                    Text("Set a daily word target, deadline, or total goal to enable the session badge.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            } else {
                editorRows
            }
        }
        .onAppear { reload() }
    }

    @ViewBuilder
    private func displayRows(_ p: SessionProgress) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            if let goal = p.wordsPerSession {
                row(label: "Today", value: "\(p.todayWords.formatted()) / \(goal.formatted())")
            }
            if let deadline = p.deadline {
                let suffix = p.daysRemaining.map { " (\($0)d)" } ?? ""
                row(label: "Deadline", value: "\(deadline)\(suffix)")
            }
            if let total = p.totalTarget {
                row(label: "Total", value: "\(p.currentTotal.formatted()) / \(total.formatted())")
            }
            if let need = p.neededPerDay {
                row(label: "Needed/day to finish", value: need.formatted())
            }
        }
    }

    private func row(label: String, value: String) -> some View {
        HStack {
            Text(label).font(.caption).foregroundStyle(.secondary)
            Spacer()
            Text(value).font(.caption).monospacedDigit()
        }
    }

    @ViewBuilder
    private var editorRows: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Words per session").font(.caption).foregroundStyle(.secondary)
                Spacer()
                TextField("1000", text: $wordsPerSession).textFieldStyle(.roundedBorder).frame(width: 100)
            }
            HStack {
                Text("Deadline").font(.caption).foregroundStyle(.secondary)
                Spacer()
                TextField("YYYY-MM-DD", text: $deadline).textFieldStyle(.roundedBorder).frame(width: 140)
            }
            HStack {
                Text("Total target").font(.caption).foregroundStyle(.secondary)
                Spacer()
                TextField("90000", text: $totalTarget).textFieldStyle(.roundedBorder).frame(width: 100)
            }
            HStack {
                Spacer()
                Button("Save") {
                    save()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
    }

    private var hasTarget: Bool {
        guard let progress else { return false }
        return progress.wordsPerSession != nil
            || progress.totalTarget != nil
            || progress.deadline != nil
    }

    private func reload() {
        progress = store.sessionProgress()
        loadFromProgress()
    }

    private func loadFromProgress() {
        guard let p = progress else { return }
        wordsPerSession = p.wordsPerSession.map(String.init) ?? ""
        deadline = p.deadline ?? ""
        totalTarget = p.totalTarget.map(String.init) ?? ""
    }

    private func save() {
        let target = SessionTarget(
            wordsPerSession: Int(wordsPerSession),
            deadline: deadline.isEmpty ? nil : deadline,
            totalTarget: Int(totalTarget)
        )
        store.updateSessionTarget(target)
        progress = store.sessionProgress()
        editing = false
    }
}
