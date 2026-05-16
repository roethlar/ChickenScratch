import Foundation
import ChiknKit

/// Round-trip checks for the format contract. Mirrors the Rust tests in
/// `crates/core/src/core/project/writer.rs`. Runs without XCTest so it
/// works in the Command Line Tools toolchain (`swift run ChiknKitChecks`).

var failures = 0

@MainActor
func check(_ condition: Bool, _ label: String, file: String = #file, line: Int = #line) {
    if condition {
        print("  ✓ \(label)")
    } else {
        print("  ✗ \(label) — \((file as NSString).lastPathComponent):\(line)")
        failures += 1
    }
}

@MainActor
func runCase(_ name: String, _ body: () throws -> Void) {
    print("• \(name)")
    do {
        try body()
    } catch {
        print("  ✗ threw: \(error)")
        failures += 1
    }
}

// MARK: - Helpers

func makeTempProjectURL() -> URL {
    let id = UUID().uuidString.lowercased()
    return FileManager.default.temporaryDirectory
        .appendingPathComponent("chikn-check-\(id).chikn")
}

func cleanup(_ url: URL) {
    try? FileManager.default.removeItem(at: url)
}

func topLevelFolderID(named name: String, in project: Project) -> String? {
    project.hierarchy.first { $0.kind == .folder && $0.name == name }?.id
}

// MARK: - Cases

runCase("createProject uses UUID IDs for required folders") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    let project = try Writer.createProject(at: url, name: "RootIDs")
    let legacyIDs = Set(["manuscript", "research", "trash"])

    for name in ["Manuscript", "Research", "Trash"] {
        guard let id = topLevelFolderID(named: name, in: project) else {
            check(false, "\(name) folder exists")
            continue
        }
        check(UUID(uuidString: id) != nil, "\(name) folder id is a UUID")
        check(id == id.lowercased(), "\(name) folder id is lowercase")
        check(!legacyIDs.contains(id), "\(name) folder id is not a legacy literal")
    }
}

runCase("legacy root parent IDs still write under UUID roots") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "LegacyAliases")
    guard let manuscriptID = topLevelFolderID(named: "Manuscript", in: project),
          let researchID = topLevelFolderID(named: "Research", in: project)
    else {
        check(false, "required root folders exist")
        return
    }

    let (p1, scene) = try Writer.createDocument(name: "Opening", parentID: "manuscript", in: project)
    project = p1
    let (p2, note) = try Writer.createDocument(name: "Clue", parentID: "research", in: project)
    project = p2

    let manuscript = project.hierarchy.first { $0.id == manuscriptID }
    let research = project.hierarchy.first { $0.id == researchID }
    check(scene.relativePath == "manuscript/opening.md", "legacy manuscript alias keeps manuscript path")
    check(note.relativePath == "research/clue.md", "legacy research alias keeps research path")
    check(manuscript?.children.contains(where: { $0.id == scene.id }) == true, "legacy manuscript alias attaches to UUID root")
    check(research?.children.contains(where: { $0.id == note.id }) == true, "legacy research alias attaches to UUID root")

    let sceneMetaURL = project.path.appendingPathComponent(scene.relativePath)
        .deletingPathExtension().appendingPathExtension("meta")
    let sceneMeta = try String(contentsOf: sceneMetaURL, encoding: .utf8)
    check(sceneMeta.contains(manuscriptID), "meta parent_id stores resolved UUID root")
}

runCase("fields map round-trip preserves arbitrary keys") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Fields")
    let (p1, doc) = try Writer.createDocument(name: "Opening", parentID: "manuscript", in: project)
    project = p1

    var meta = doc.meta
    meta.fields = [
        "pov_character": .string("sarah"),
        "duration_minutes": .int(45),
        "threads": .array([.string("main-plot"), .string("romance")]),
        "world_state": .dict([
            "scale": .string("medium"),
            "year": .int(1987),
        ]),
    ]
    project = try Writer.saveDocumentMeta(id: doc.id, meta: meta, in: project)

    let reread = try Reader.readProject(at: url)
    let back = reread.documents[doc.id]
    check(back?.meta.fields["pov_character"]?.asString == "sarah", "string field round-trips")
    check(back?.meta.fields["duration_minutes"]?.asInt == 45, "int field round-trips")
    check(back?.meta.fields["threads"]?.asStringArray == ["main-plot", "romance"], "string array field round-trips")
    if case .dict(let world) = back?.meta.fields["world_state"] {
        check(world["scale"]?.asString == "medium", "nested string round-trips")
        check(world["year"]?.asInt == 1987, "nested int round-trips")
    } else {
        check(false, "nested mapping shape preserved")
    }
}

runCase("empty fields map writes a clean .meta") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Clean")
    let (p1, doc) = try Writer.createDocument(name: "Plain", parentID: "manuscript", in: project)
    project = p1

    var meta = doc.meta
    meta.fields = [:]
    _ = try Writer.saveDocumentMeta(id: doc.id, meta: meta, in: project)

    let metaURL = url.appendingPathComponent(doc.relativePath)
        .deletingPathExtension().appendingPathExtension("meta")
    let metaText = try String(contentsOf: metaURL, encoding: .utf8)
    check(!metaText.contains("fields:"), "empty fields map skipped on write")
}

runCase("foreign keys inside fields survive a read/write/read cycle") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Preserve")
    let (p1, doc) = try Writer.createDocument(name: "Session 7", parentID: "manuscript", in: project)
    project = p1

    let metaURL = url.appendingPathComponent(doc.relativePath)
        .deletingPathExtension().appendingPathExtension("meta")
    let existing = try String(contentsOf: metaURL, encoding: .utf8)
    let injected = existing.trimmingCharacters(in: .whitespacesAndNewlines)
        + "\nfields:\n  ttrpg_session_date: 2026-04-23\n  ttrpg_encounter_cr: 12\n"
    try injected.write(to: metaURL, atomically: true, encoding: .utf8)

    let reloaded = try Reader.readProject(at: url)
    let reloadedDoc = reloaded.documents[doc.id]
    check(reloadedDoc?.meta.fields["ttrpg_session_date"]?.asString == "2026-04-23", "foreign string survives initial read")
    check(reloadedDoc?.meta.fields["ttrpg_encounter_cr"]?.asInt == 12, "foreign int survives initial read")

    var meta = reloadedDoc!.meta
    meta.synopsis = "Dragon arrives."
    _ = try Writer.saveDocumentMeta(id: reloadedDoc!.id, meta: meta, in: reloaded)

    let final = try Reader.readProject(at: url)
    let finalDoc = final.documents[doc.id]
    check(finalDoc?.meta.fields["ttrpg_session_date"]?.asString == "2026-04-23", "foreign string survives writer pass")
    check(finalDoc?.meta.fields["ttrpg_encounter_cr"]?.asInt == 12, "foreign int survives writer pass")
    check(finalDoc?.meta.synopsis == "Dragon arrives.", "synopsis written through")
}

runCase("threads.yaml round-trips") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Threads")
    project = try Writer.createThread(name: "Main Plot", color: "#3b82f6",
                                      description: "Sarah uncovers the truth.", in: project)
    project = try Writer.createThread(name: "Sarah & Marcus", color: "#ef4444", in: project)

    let reread = try Reader.readProject(at: url)
    check(reread.threads.count == 2, "thread count preserved")
    check(reread.threads[0].id == "main-plot", "first thread id derived from name")
    check(reread.threads[0].color == "#3b82f6", "color preserved")
    check(reread.threads[1].name == "Sarah & Marcus", "name preserved")
    check(reread.threads[1].description == nil, "missing description stays nil")
}

runCase("missing threads.yaml yields empty thread list") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    let project = try Writer.createProject(at: url, name: "NoThreads")
    let reread = try Reader.readProject(at: project.path)
    check(reread.threads.isEmpty, "empty when no threads.yaml")
}

runCase("deleting a thread strips dangling refs from scenes") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Strip")
    project = try Writer.createThread(name: "Main", in: project)
    let (p1, doc) = try Writer.createDocument(name: "Scene", parentID: "manuscript", in: project)
    project = p1
    var meta = doc.meta
    meta.fields["threads"] = .array([.string("main")])
    project = try Writer.saveDocumentMeta(id: doc.id, meta: meta, in: project)

    var reread = try Reader.readProject(at: url)
    check(reread.documents[doc.id]?.meta.fields["threads"]?.asStringArray == ["main"], "ref present before delete")

    project = try Writer.deleteThread(id: "main", in: reread)
    reread = try Reader.readProject(at: url)
    check(reread.threads.isEmpty, "thread removed")
    check(reread.documents[doc.id]?.meta.fields["threads"] == nil, "doc ref stripped")
}

runCase("session_target round-trips and is omitted when empty") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Targets")

    let bareYamlURL = project.path.appendingPathComponent("project.yaml")
    let bareYaml = try String(contentsOf: bareYamlURL, encoding: .utf8)
    check(!bareYaml.contains("session_target"), "absent target omits the YAML key")

    project.metadata.sessionTarget = SessionTarget(
        wordsPerSession: 1000,
        deadline: "2026-12-31",
        totalTarget: 90_000
    )
    try Writer.touchProject(project)

    let reread = try Reader.readProject(at: url)
    let st = reread.metadata.sessionTarget
    check(st?.wordsPerSession == 1000, "wordsPerSession round-trips")
    check(st?.deadline == "2026-12-31", "deadline round-trips")
    check(st?.totalTarget == 90_000, "totalTarget round-trips")
}

runCase("created entities are loaded by the reader") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Entities")
    let (p1, charDoc) = try Writer.createEntity(kind: .character, name: "Sarah Bennett", in: project)
    project = p1
    let (_, locDoc) = try Writer.createEntity(kind: .location, name: "Motel Room 12", in: project)

    let reread = try Reader.readProject(at: url)
    let char = reread.documents[charDoc.id]
    check(char?.relativePath == "characters/sarah-bennett.md", "character path")
    check(char?.meta.fields["entity_kind"]?.asString == "character", "character entity_kind tag")

    let loc = reread.documents[locDoc.id]
    check(loc?.relativePath == "locations/motel-room-12.md", "location path")
    check(loc?.meta.fields["entity_kind"]?.asString == "location", "location entity_kind tag")
}

// MARK: - Slice B: drafts, per-doc history, dangling refs

runCase("validateReferences flags dangling pov / location / thread refs") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Refs")
    let (p1, sarah) = try Writer.createEntity(kind: .character, name: "Sarah Bennett", in: project)
    project = p1
    project = try Writer.createThread(name: "Main Plot", in: project)

    // Reference one valid character + one missing one + a missing thread.
    let (p2, scene) = try Writer.createDocument(name: "Scene 1", parentID: "manuscript", in: project)
    project = p2
    var meta = scene.meta
    meta.fields["pov_character"] = .string(entitySlugOf(sarah))
    meta.fields["location"] = .string("missing-place")
    meta.fields["threads"] = .array([.string("main-plot"), .string("ghost-thread")])
    project = try Writer.saveDocumentMeta(id: scene.id, meta: meta, in: project)

    let dangling = References.validate(project)
    let missing = Set(dangling.map { "\($0.field)|\($0.missingID)" })
    check(missing.contains("location|missing-place"), "missing location flagged")
    check(missing.contains("threads|ghost-thread"), "missing thread flagged")
    check(!missing.contains("pov_character|\(entitySlugOf(sarah))"), "valid pov not flagged")
}

runCase("createDraft + listDrafts + switchDraft + mergeDraft round-trip") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Drafts")
    _ = try Git.saveRevision(message: "Seed", in: project.path)

    // The default branch name varies (`main` vs `master`) by git config.
    // Capture whatever the seeded project lives on so the test stays portable.
    let defaultBranch = try Git.listDrafts(in: project.path)
        .first(where: { $0.isActive })?.name ?? "main"

    try Git.createDraft(name: "draft-2", in: project.path)
    var drafts = try Git.listDrafts(in: project.path)
    check(drafts.count == 2, "two drafts after create (\(defaultBranch) + draft-2)")
    let active = drafts.first(where: { $0.isActive })?.name
    check(active == "draft-2", "draft-2 is active after createDraft")

    // Add a new doc on draft-2 so the merge has something to bring back.
    let (p1, draftDoc) = try Writer.createDocument(name: "Draft Only", parentID: "manuscript", in: project)
    project = p1
    try draftDoc.content.write(
        to: project.path.appendingPathComponent(draftDoc.relativePath),
        atomically: true, encoding: .utf8
    )
    _ = try Git.saveRevision(message: "Add draft-only doc", in: project.path)

    // Switch back to the seeded branch; the new doc must not be present.
    try Git.switchDraft(name: defaultBranch, in: project.path)
    let postSwitch = try Reader.readProject(at: project.path)
    check(postSwitch.documents[draftDoc.id] == nil, "draft-only doc absent on \(defaultBranch)")

    // Merge draft-2 — the new doc should reappear.
    try Git.mergeDraft(name: "draft-2", in: project.path)
    let postMerge = try Reader.readProject(at: project.path)
    check(postMerge.documents[draftDoc.id] != nil, "draft-only doc present after merge")
    drafts = try Git.listDrafts(in: project.path)
    check(drafts.first(where: { $0.isActive })?.name == defaultBranch, "\(defaultBranch) is active after merge")
}

runCase("documentHistory + restoreDocument round-trip") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "DocHistory")
    let (p1, doc) = try Writer.createDocument(name: "Chapter 1", parentID: "manuscript", in: project)
    project = p1

    // First saved revision — initial empty body.
    try "Original body.".write(
        to: project.path.appendingPathComponent(doc.relativePath),
        atomically: true, encoding: .utf8
    )
    _ = try Git.saveRevision(message: "First draft", in: project.path)

    // Capture the SHA of that first state.
    let history1 = try Git.documentHistory(documentPath: doc.relativePath, in: project.path)
    check(history1.count >= 1, "at least one historical commit for the doc")
    let firstSHA = history1.first?.id ?? ""

    // Second revision — different content.
    try "Revised body.".write(
        to: project.path.appendingPathComponent(doc.relativePath),
        atomically: true, encoding: .utf8
    )
    _ = try Git.saveRevision(message: "Revise body", in: project.path)

    let history2 = try Git.documentHistory(documentPath: doc.relativePath, in: project.path)
    check(history2.count >= 2, "second commit visible in per-doc history")

    // Restore to the first state. Working tree should reflect "Original body."
    _ = try Git.restoreDocument(documentPath: doc.relativePath, commitHash: firstSHA, in: project.path)
    let restored = try String(
        contentsOf: project.path.appendingPathComponent(doc.relativePath),
        encoding: .utf8
    )
    check(restored == "Original body.", "restored content matches historical blob")

    // Restore should produce a new commit (forward-only; never rewrites).
    let history3 = try Git.documentHistory(documentPath: doc.relativePath, in: project.path)
    check(history3.count >= 3, "restore produces a new commit, not a rewind")
}

// MARK: - Slice C: stats, writing history, hierarchy ops

runCase("wordCount strips HTML and skips markdown punctuation") {
    // Tauri injects a space when closing an HTML tag, so "</em>!" splits as
    // "world" + "!" — three tokens total. We match that behavior to keep
    // counts consistent across frontends.
    check(Stats.wordCount(markdown: "Hello <em>world</em>!") == 3, "tags split, prose punctuation counts")
    check(Stats.wordCount(markdown: "# Heading\n\n* one\n* two") == 3, "headings/bullets don't count tokens")
    check(Stats.wordCount(markdown: "") == 0, "empty content")
}

runCase("projectStats segments manuscript vs total") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Stats")
    let (p1, msDoc) = try Writer.createDocument(name: "Chapter 1", parentID: "manuscript", in: project)
    project = p1
    try "one two three four five".write(
        to: project.path.appendingPathComponent(msDoc.relativePath),
        atomically: true, encoding: .utf8
    )
    let (p2, researchDoc) = try Writer.createDocument(name: "Note", parentID: "research", in: project)
    project = p2
    try "alpha beta".write(
        to: project.path.appendingPathComponent(researchDoc.relativePath),
        atomically: true, encoding: .utf8
    )

    project = try Reader.readProject(at: url)
    let s = Stats.projectStats(project)
    check(s.manuscriptWords == 5, "manuscript words counted")
    check(s.totalWords == 7, "total words includes research")
    check(s.totalDocs == 2, "doc count")
}

runCase("writing history captures start_words and tracks today's net") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    let project = try Writer.createProject(at: url, name: "History")

    // First record: 100 words. start_words should equal 100 — there's
    // nothing earlier today to baseline from.
    var history = try Stats.recordDailyWords(100, in: project.path)
    check(history.entries.count == 1, "one entry created")
    check(history.entries[0].words == 100, "current count recorded")
    check(history.entries[0].startWords == 100, "start_words captured on first call")

    // Second record same day: count climbed to 250. start_words must NOT
    // overwrite — that's how today's net (150) is computed downstream.
    history = try Stats.recordDailyWords(250, in: project.path)
    check(history.entries[0].words == 250, "current count updated")
    check(history.entries[0].startWords == 100, "start_words preserved across calls")
}

runCase("sessionProgress reports today's net and needed/day") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Progress")

    // Manuscript currently at 5000 words, target is 90k by 2099-12-31.
    // Pretend the day started at 4000.
    project.metadata.sessionTarget = SessionTarget(
        wordsPerSession: 1000,
        deadline: "2099-12-31",
        totalTarget: 90_000
    )
    try Writer.touchProject(project)
    _ = try Stats.recordDailyWords(4000, in: project.path)

    // Write a manuscript doc with ~5000 words so projectStats matches.
    let (p1, doc) = try Writer.createDocument(name: "Body", parentID: "manuscript", in: project)
    project = p1
    let body = String(repeating: "word ", count: 5000)
    try body.write(
        to: project.path.appendingPathComponent(doc.relativePath),
        atomically: true, encoding: .utf8
    )

    // Bump today's record to the new total to capture the diff.
    _ = try Stats.recordDailyWords(5000, in: project.path)

    project = try Reader.readProject(at: url)
    let p = Stats.sessionProgress(project)
    check(p.currentTotal == 5000, "current manuscript total")
    check(p.todayWords == 1000, "today net is current - start")
    check(p.wordsPerSession == 1000, "session goal preserved")
    check(p.totalTarget == 90_000, "total target preserved")
    check(p.daysRemaining ?? 0 > 0, "deadline in the future yields positive days")
    check(p.neededPerDay != nil, "needed/day computed when target+deadline present")
}

runCase("deleteNode removes a document from hierarchy and disk") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Delete")
    let (p1, doc) = try Writer.createDocument(name: "Goner", parentID: "manuscript", in: project)
    project = p1
    let docURL = project.path.appendingPathComponent(doc.relativePath)
    let metaURL = docURL.deletingPathExtension().appendingPathExtension("meta")
    check(FileManager.default.fileExists(atPath: docURL.path), "doc file present before delete")

    project = try Writer.deleteNode(id: doc.id, in: project)
    check(!FileManager.default.fileExists(atPath: docURL.path), ".md removed from disk")
    check(!FileManager.default.fileExists(atPath: metaURL.path), ".meta removed from disk")
    check(project.documents[doc.id] == nil, "doc removed from project.documents")
}

runCase("moveNode relocates a doc into Trash without deleting files") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Trash")
    let (p1, doc) = try Writer.createDocument(name: "Drafty", parentID: "manuscript", in: project)
    project = p1
    let trashID = project.hierarchy.first(where: { $0.kind == .folder && $0.name == "Trash" })?.id ?? ""

    project = try Writer.moveNode(id: doc.id, newParentID: trashID, in: project)
    let docURL = project.path.appendingPathComponent(doc.relativePath)
    check(FileManager.default.fileExists(atPath: docURL.path), "files stay on disk after Trash")

    // Hierarchy now lists the doc under Trash.
    let trashFolder = project.hierarchy.first(where: { $0.id == trashID })
    check(trashFolder?.children.contains(where: { $0.id == doc.id }) == true, "doc now under Trash")
}

runCase("reorderNode swaps siblings within a parent") {
    let url = makeTempProjectURL()
    defer { cleanup(url) }

    var project = try Writer.createProject(at: url, name: "Reorder")
    let (p1, a) = try Writer.createDocument(name: "Alpha", parentID: "manuscript", in: project)
    project = p1
    let (p2, b) = try Writer.createDocument(name: "Beta", parentID: "manuscript", in: project)
    project = p2
    let (p3, c) = try Writer.createDocument(name: "Gamma", parentID: "manuscript", in: project)
    project = p3

    // Manuscript children should be [Alpha, Beta, Gamma] now.
    func manuscriptChildIDs(_ p: Project) -> [String] {
        p.hierarchy.first(where: { $0.name == "Manuscript" })?.children.map(\.id) ?? []
    }
    check(manuscriptChildIDs(project) == [a.id, b.id, c.id], "initial sibling order")

    // Move Gamma up to index 0.
    project = try Writer.reorderNode(id: c.id, newIndex: 0, in: project)
    check(manuscriptChildIDs(project) == [c.id, a.id, b.id], "Gamma moved to front")
}

if failures == 0 {
    print("\nAll checks passed.")
    exit(0)
} else {
    print("\n\(failures) check(s) failed.")
    exit(1)
}

private func entitySlugOf(_ document: Document) -> String {
    let last = (document.relativePath as NSString).lastPathComponent
    return (last as NSString).deletingPathExtension
}
