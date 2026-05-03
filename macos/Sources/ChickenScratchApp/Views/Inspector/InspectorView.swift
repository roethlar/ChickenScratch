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

/// Novelist-UI convention keys — see docs/UI_CONVENTIONS_NOVELIST.md.
/// The format itself stores them in `DocumentMeta.fields`; these constants
/// are the names this repo's novelist UIs agree on.
private enum NovelistKey {
    static let povCharacter = "pov_character"
    static let location = "location"
    static let storyTime = "story_time"
    static let durationMinutes = "duration_minutes"
    static let threads = "threads"
    static let charactersInScene = "characters_in_scene"
    static let entityKind = "entity_kind"
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

    // Scene section (novelist convention; persisted in `meta.fields`)
    @State private var povCharacter: String = ""
    @State private var location: String = ""
    @State private var storyTime: String = ""
    @State private var durationMinutes: String = ""
    @State private var threads: [String] = []
    @State private var charactersInScene: String = ""
    @State private var sceneExpanded: Bool = false

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

                    sceneSection

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

    // MARK: - Scene section

    @ViewBuilder
    private var sceneSection: some View {
        DisclosureGroup(isExpanded: $sceneExpanded) {
            VStack(alignment: .leading, spacing: 12) {
                entityField(
                    title: "POV Character",
                    placeholder: "sarah-bennett",
                    text: $povCharacter,
                    kind: .character
                )
                entityField(
                    title: "Location",
                    placeholder: "motel-room-12",
                    text: $location,
                    kind: .location
                )
                field(title: "Story Time") {
                    TextField("Day 3, 22:30 — or ISO date", text: $storyTime)
                        .textFieldStyle(.roundedBorder)
                        .onChange(of: storyTime) { _, _ in saveMeta() }
                }
                field(title: "Duration (minutes)") {
                    TextField("45", text: $durationMinutes)
                        .textFieldStyle(.roundedBorder)
                        .onChange(of: durationMinutes) { _, _ in saveMeta() }
                }
                threadsField
                field(title: "Other characters") {
                    TextField("comma-separated slugs", text: $charactersInScene)
                        .textFieldStyle(.roundedBorder)
                        .onChange(of: charactersInScene) { _, _ in saveMeta() }
                }
            }
            .padding(.top, 8)
        } label: {
            Text("Scene".uppercased())
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    private func entityField(
        title: String,
        placeholder: String,
        text: Binding<String>,
        kind: EntityKind
    ) -> some View {
        let entities = store.entities(of: kind)
        let currentSlug = text.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines)
        let known = entities.contains { entitySlug($0) == currentSlug }

        VStack(alignment: .leading, spacing: 6) {
            Text(title.uppercased()).font(.caption2.weight(.semibold)).foregroundStyle(.secondary)

            HStack(spacing: 6) {
                TextField(placeholder, text: text)
                    .textFieldStyle(.roundedBorder)
                    .onChange(of: text.wrappedValue) { _, _ in saveMeta() }

                Menu {
                    if entities.isEmpty {
                        Text("No \(kind.rawValue)s yet").foregroundStyle(.secondary)
                    } else {
                        ForEach(entities) { entity in
                            Button(entity.name) {
                                text.wrappedValue = entitySlug(entity)
                                saveMeta()
                            }
                        }
                    }
                    if !currentSlug.isEmpty && !known {
                        Divider()
                        Button("Create \(kind.rawValue) \"\(currentSlug)\"") {
                            createEntity(kind: kind, displayName: currentSlug, into: text)
                        }
                    }
                } label: {
                    Image(systemName: "person.crop.circle.badge.plus")
                }
                .menuStyle(.borderlessButton)
                .frame(width: 28)
            }

            if !currentSlug.isEmpty && !known {
                Button {
                    createEntity(kind: kind, displayName: currentSlug, into: text)
                } label: {
                    Label("Create \(kind.rawValue) \"\(currentSlug)\"", systemImage: "plus.circle")
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .foregroundStyle(.tint)
            }
        }
    }

    @ViewBuilder
    private var threadsField: some View {
        let available = store.project?.threads ?? []
        let byID = Dictionary(uniqueKeysWithValues: available.map { ($0.id, $0) })
        let unselected = available.filter { !threads.contains($0.id) }

        VStack(alignment: .leading, spacing: 6) {
            Text("Threads".uppercased())
                .font(.caption2.weight(.semibold))
                .foregroundStyle(.secondary)

            HStack(spacing: 4) {
                if threads.isEmpty {
                    Text("None")
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                } else {
                    ForEach(threads, id: \.self) { id in
                        threadChip(id: id, thread: byID[id])
                    }
                }
                Spacer()
                Menu {
                    if unselected.isEmpty && available.isEmpty {
                        Text("No threads yet").foregroundStyle(.secondary)
                    } else if unselected.isEmpty {
                        Text("All added").foregroundStyle(.secondary)
                    } else {
                        ForEach(unselected) { thread in
                            Button(thread.name) {
                                threads.append(thread.id)
                                saveMeta()
                            }
                        }
                    }
                } label: {
                    Image(systemName: "plus.circle")
                }
                .menuStyle(.borderlessButton)
                .frame(width: 28)
            }
        }
    }

    @ViewBuilder
    private func threadChip(id: String, thread: ChiknKit.Thread?) -> some View {
        HStack(spacing: 4) {
            if let color = thread?.color, let nsColor = colorFromHex(color) {
                Circle().fill(Color(nsColor: nsColor)).frame(width: 8, height: 8)
            } else {
                Circle().fill(Color.secondary).frame(width: 8, height: 8)
            }
            Text(thread?.name ?? id).font(.caption)
            Button {
                threads.removeAll(where: { $0 == id })
                saveMeta()
            } label: {
                Image(systemName: "xmark").font(.caption2)
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 3)
        .background(.quaternary.opacity(0.4), in: Capsule())
    }

    // MARK: - Load / save

    private func loadFields() {
        synopsis   = document.meta.synopsis ?? ""
        label      = document.meta.label ?? ""
        status     = document.meta.status ?? ""
        keywords   = document.meta.keywords.joined(separator: ", ")
        includeInCompile = document.meta.includeInCompile
        wordTarget = document.meta.wordCountTarget.map(String.init) ?? ""

        let f = document.meta.fields
        povCharacter = f[NovelistKey.povCharacter]?.asString ?? ""
        location = f[NovelistKey.location]?.asString ?? ""
        storyTime = f[NovelistKey.storyTime]?.asString ?? ""
        durationMinutes = f[NovelistKey.durationMinutes]?.asInt.map(String.init) ?? ""
        threads = f[NovelistKey.threads]?.asStringArray ?? []
        charactersInScene = (f[NovelistKey.charactersInScene]?.asStringArray ?? []).joined(separator: ", ")

        sceneExpanded = !povCharacter.isEmpty
            || !location.isEmpty
            || !storyTime.isEmpty
            || !durationMinutes.isEmpty
            || !threads.isEmpty
            || !charactersInScene.isEmpty
    }

    private func saveMeta() {
        let kws = keywords.split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }

        var fields = document.meta.fields
        // Preserve any keys we don't model here (entity_kind, foreign keys, etc.)
        // by only touching the novelist-convention slots.
        setField(&fields, key: NovelistKey.povCharacter, string: povCharacter)
        setField(&fields, key: NovelistKey.location, string: location)
        setField(&fields, key: NovelistKey.storyTime, string: storyTime)
        setField(&fields, key: NovelistKey.durationMinutes, int: Int(durationMinutes))
        setField(&fields, key: NovelistKey.threads, list: threads)
        setField(
            &fields,
            key: NovelistKey.charactersInScene,
            list: charactersInScene
                .split(separator: ",")
                .map { $0.trimmingCharacters(in: .whitespaces) }
                .filter { !$0.isEmpty }
        )

        let meta = DocumentMeta(
            synopsis: synopsis.isEmpty ? nil : synopsis,
            label: label.isEmpty ? nil : label,
            status: status.isEmpty ? nil : status,
            keywords: kws,
            includeInCompile: includeInCompile,
            wordCountTarget: Int(wordTarget),
            compileOrder: document.meta.compileOrder,
            fields: fields
        )
        store.saveDocumentMeta(id: document.id, meta: meta)
    }

    private func setField(_ fields: inout [String: YAMLValue], key: String, string: String) {
        let trimmed = string.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { fields.removeValue(forKey: key) }
        else { fields[key] = .string(trimmed) }
    }

    private func setField(_ fields: inout [String: YAMLValue], key: String, int: Int?) {
        if let v = int, v > 0 { fields[key] = .int(v) }
        else { fields.removeValue(forKey: key) }
    }

    private func setField(_ fields: inout [String: YAMLValue], key: String, list: [String]) {
        if list.isEmpty { fields.removeValue(forKey: key) }
        else { fields[key] = .array(list.map { .string($0) }) }
    }

    private func createEntity(kind: EntityKind, displayName: String, into binding: Binding<String>) {
        guard let doc = store.createEntity(kind: kind, name: displayName) else { return }
        binding.wrappedValue = entitySlug(doc)
        saveMeta()
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

// MARK: - Helpers

/// Map an entity Document to its filename slug — the value persisted in
/// `pov_character` / `location` / `characters_in_scene`.
func entitySlug(_ document: Document) -> String {
    let last = (document.relativePath as NSString).lastPathComponent
    return ((last as NSString).deletingPathExtension)
}

/// Parse `#rrggbb` (with or without alpha) into NSColor. Falls back to nil so
/// the caller can render a neutral default.
func colorFromHex(_ hex: String) -> NSColor? {
    var s = hex.trimmingCharacters(in: .whitespacesAndNewlines)
    if s.hasPrefix("#") { s.removeFirst() }
    guard s.count == 6 || s.count == 8 else { return nil }
    var v: UInt64 = 0
    guard Scanner(string: s).scanHexInt64(&v) else { return nil }
    let r, g, b, a: CGFloat
    if s.count == 6 {
        r = CGFloat((v & 0xFF0000) >> 16) / 255
        g = CGFloat((v & 0x00FF00) >>  8) / 255
        b = CGFloat( v & 0x0000FF       ) / 255
        a = 1
    } else {
        r = CGFloat((v & 0xFF000000) >> 24) / 255
        g = CGFloat((v & 0x00FF0000) >> 16) / 255
        b = CGFloat((v & 0x0000FF00) >>  8) / 255
        a = CGFloat( v & 0x000000FF       ) / 255
    }
    return NSColor(srgbRed: r, green: g, blue: b, alpha: a)
}

