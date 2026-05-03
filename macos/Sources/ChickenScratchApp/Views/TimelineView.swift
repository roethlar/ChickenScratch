import ChiknKit
import SwiftUI

/// Story-time timeline. Reads `story_time`, `duration_minutes`,
/// `pov_character`, and `threads` from each document's `fields` map and lays
/// scenes out on a horizontal axis. Lane modes mirror the Tauri view:
///   - POV — one lane per POV character
///   - Thread — one lane per thread (a scene appears in every thread it tags)
///   - Single — one chronological track
struct TimelineView: View {
    @Environment(ProjectStore.self) private var store
    @State private var laneMode: LaneMode = .pov

    enum LaneMode: String, CaseIterable, Identifiable {
        case pov = "POV"
        case thread = "Thread"
        case single = "Single"
        var id: String { rawValue }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Timeline").font(.headline)
                Spacer()
                Picker("", selection: $laneMode) {
                    ForEach(LaneMode.allCases) { m in Text(m.rawValue).tag(m) }
                }
                .pickerStyle(.segmented)
                .frame(width: 220)
            }
            .padding(.horizontal, 16).padding(.vertical, 10)

            Divider()

            content
        }
    }

    @ViewBuilder
    private var content: some View {
        if let project = store.project {
            let scenes = extractScenes(from: project)
            if scenes.isEmpty {
                emptyState
            } else {
                timeline(scenes: scenes, project: project)
            }
        } else {
            emptyState
        }
    }

    private var emptyState: some View {
        ContentUnavailableView(
            "No scenes with a Story Time",
            systemImage: "calendar",
            description: Text("Add a Story Time to scenes in the Inspector to see them here.")
        )
    }

    @ViewBuilder
    private func timeline(scenes: [TimelineScene], project: Project) -> some View {
        let bounds = (
            min: scenes.map(\.time).min() ?? 0,
            max: scenes.map(\.time).max() ?? 1
        )
        let span = max(bounds.max - bounds.min, 1)
        let lanes = group(scenes: scenes, mode: laneMode)
        let threadIndex = Dictionary(uniqueKeysWithValues: project.threads.map { ($0.id, $0) })

        ScrollView(.vertical) {
            VStack(alignment: .leading, spacing: 12) {
                ForEach(lanes, id: \.0) { (laneName, laneScenes) in
                    laneRow(name: laneName, scenes: laneScenes, span: span, low: bounds.min, threadIndex: threadIndex)
                }
            }
            .padding(16)
        }
    }

    @ViewBuilder
    private func laneRow(
        name: String,
        scenes: [TimelineScene],
        span: Double,
        low: Double,
        threadIndex: [String: ChiknKit.Thread]
    ) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(name).font(.caption.weight(.semibold)).foregroundStyle(.secondary)
            GeometryReader { geo in
                ZStack(alignment: .topLeading) {
                    Rectangle()
                        .fill(.quaternary.opacity(0.3))
                        .frame(height: 1)
                        .offset(y: 30)
                    ForEach(scenes) { scene in
                        let x = geo.size.width * CGFloat((scene.time - low) / max(span, 1))
                        Button {
                            store.selectedNodeID = scene.doc.id
                        } label: {
                            chip(scene: scene, threadIndex: threadIndex)
                        }
                        .buttonStyle(.plain)
                        .position(x: max(60, min(geo.size.width - 60, x)), y: 30)
                    }
                }
            }
            .frame(height: 70)
        }
    }

    @ViewBuilder
    private func chip(scene: TimelineScene, threadIndex: [String: ChiknKit.Thread]) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(scene.doc.name).font(.caption.weight(.medium)).lineLimit(1)
            Text(scene.displayTime).font(.system(size: 9, design: .monospaced)).foregroundStyle(.secondary)
            if !scene.threads.isEmpty {
                HStack(spacing: 2) {
                    ForEach(scene.threads.prefix(4), id: \.self) { id in
                        if let nsColor = threadIndex[id]?.color.flatMap(colorFromHex) {
                            Circle().fill(Color(nsColor: nsColor)).frame(width: 6, height: 6)
                        } else {
                            Circle().fill(Color.secondary).frame(width: 6, height: 6)
                        }
                    }
                }
            }
        }
        .padding(.horizontal, 8).padding(.vertical, 5)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 6))
        .help(scene.doc.meta.synopsis ?? scene.doc.name)
    }

    // MARK: - Scene extraction & grouping

    private struct TimelineScene: Identifiable {
        let doc: Document
        let time: Double
        let displayTime: String
        let pov: String?
        let threads: [String]
        var id: String { doc.id }
    }

    private func extractScenes(from project: Project) -> [TimelineScene] {
        var out: [TimelineScene] = []
        for doc in project.documents.values {
            guard let raw = doc.meta.fields["story_time"]?.asString else { continue }
            guard let (time, display) = parseStoryTime(raw) else { continue }
            let pov = doc.meta.fields["pov_character"]?.asString
            let threads = doc.meta.fields["threads"]?.asStringArray ?? []
            out.append(TimelineScene(doc: doc, time: time, displayTime: display, pov: pov, threads: threads))
        }
        return out.sorted { $0.time < $1.time }
    }

    private func group(scenes: [TimelineScene], mode: LaneMode) -> [(String, [TimelineScene])] {
        var lanes: [String: [TimelineScene]] = [:]
        var unplaced: [TimelineScene] = []
        for s in scenes {
            switch mode {
            case .single:
                lanes["Chronological", default: []].append(s)
            case .pov:
                let key = s.pov?.isEmpty == false ? s.pov! : "Unknown POV"
                lanes[key, default: []].append(s)
            case .thread:
                if s.threads.isEmpty {
                    unplaced.append(s)
                } else {
                    for t in s.threads {
                        lanes[t, default: []].append(s)
                    }
                }
            }
        }
        var ordered = lanes.sorted { $0.key.localizedCaseInsensitiveCompare($1.key) == .orderedAscending }
        if !unplaced.isEmpty {
            ordered.append((key: "Unplaced", value: unplaced))
        }
        return ordered.map { ($0.key, $0.value) }
    }

    /// Parse a `story_time` field. ISO 8601 → seconds since epoch; otherwise
    /// fall back to the leading integer in the string for relative ordering;
    /// otherwise treat as alphabetical (no ordering, no rendering).
    private func parseStoryTime(_ raw: String) -> (Double, String)? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }

        // Full ISO 8601 datetime first, then date-only.
        let isoDateTime = Date.ISO8601FormatStyle()
        if let date = try? isoDateTime.parse(trimmed) {
            return (date.timeIntervalSince1970, trimmed)
        }
        let dateOnly = Date.ISO8601FormatStyle().year().month().day()
        if let date = try? dateOnly.parse(trimmed) {
            return (date.timeIntervalSince1970, trimmed)
        }

        // Leading integer fallback ("Day 3, 22:30" → 3 for ordering).
        if let match = trimmed.firstMatch(of: /(\d+)/),
           let n = Int(match.1) {
            return (Double(n), trimmed)
        }

        return nil
    }
}
