# Tier 1 — Novel Structure

**Priority:** v1.2, highest leverage
**Status:** Planned
**Format impact:** Yes — `.chikn` schema extensions for scene metadata, new top-level `characters/` and `locations/` folders, new `threads.yaml`
**Depends on:** Core + Tauri frontend first; other frontends follow

Three interrelated features that turn `.chikn` from a "folder of markdown with a binder" into a proper novelist's data model. Scenes gain typed metadata, characters and locations become first-class referenceable entities, and plot threads let writers tag which scene advances which storyline. Every Tier 2 and Tier 3 feature (timeline, collections, cross-refs) becomes trivial once these three land.

---

## 1. Scene-level structured metadata

### Why

Writers routinely want to slice a manuscript structurally — "show me all POV:Sarah scenes," "which scenes happen at the motel," "how much time does Act 2 cover." Today `.chikn` has free-form label/status/keywords; writers end up using keywords like `pov:sarah` as a convention, but nothing validates them and no query can trust them.

Peer tools with this: bibisco, yWriter, oStorybook. All make POV, location, and scene duration first-class fields on scenes.

### Format changes

Optional additions to each scene's `.meta` file. All fields are optional; projects without them continue to work.

```yaml
# Existing fields continue to work unchanged.
id: 01234-abcd
name: Shelly meets Corn
synopsis: First real encounter at the motel.
# ── New optional fields ─────────────────────────────
pov_character: sarah-bennett     # id/slug reference into characters/
location: motel-room-12          # id/slug reference into locations/
story_time: "Day 3, 22:30"       # free-form string — ISO or prose
duration_minutes: 45             # integer; null = unknown
threads: [main-plot, romance]    # list of thread ids from threads.yaml
characters_in_scene:             # optional, beyond the POV character
  - marcus-rivera
  - kelly-chen
```

Backward compat: readers ignore unknown fields; writers preserve them through round-trips. Invalid references (pointing at deleted characters) don't error — the UI shows them as "Unknown" and offers to clear them.

### UX

**Inspector gains a "Scene" section** (for documents under `manuscript/`):
- **POV Character** — dropdown populated from `characters/`, plus "None" and "Other/typed"
- **Location** — dropdown from `locations/`
- **Story Time** — text input, free-form
- **Duration** — numeric input, minutes
- **Threads** — multi-select chips from `threads.yaml`
- **Other characters** — multi-select chips

The binder shows small color-coded dots next to each scene — one per thread membership, one badge for POV character. Hover to see.

### Implementation

- **Rust core (`crates/core/src/core/project/`):** extend `DocumentMeta` struct with the new optional fields; reader parses them (flexible, tolerates missing); writer round-trips them.
- **Validation:** a new `validate_references(&Project)` helper returns a list of dangling refs per scene. Non-fatal; surfaced in UI as "2 scenes reference a deleted character."
- **Tauri:** no new commands — metadata already flows through `update_document_metadata`.
- **UI (`ui/src/components/inspector/`):** expand Inspector.tsx with the new fields, driven by `project.characters` / `project.locations` / `project.threads` fetched via new commands (see below).

### Scope limits

- No enforcement that `pov_character` appears in `characters_in_scene` — lists stay independent.
- No automatic character detection in prose (v1.3 AI candidate).
- No validation at save time; invalid refs allowed and flagged later.

### Open questions

- Slug-based refs vs UUID? Slugs are human-readable in YAML and git diffs; UUIDs survive renames. Proposal: use the entity's id (UUID) as the canonical ref, but allow writers to type a slug and resolve it at save.
- Should `characters_in_scene` be auto-populated when a writer picks a POV? Proposal: no — manual control; POV is always included in queries regardless.

---

## 2. Characters and locations as first-class entities

### Why

Novelists maintain character bibles and location descriptions. Today in `.chikn` these live as free-form markdown in `research/`, with "Character Sheet" templates as a nudge. That's enough to write notes, but not enough for cross-referencing from scenes or building timelines by POV.

Peer tools with this: bibisco, oStorybook, yWriter. Plume Creator and Scrivener treat them as regular documents with templates (our current state).

The ask is **minimal structured identity** + **cross-referencing**, not deep psychology forms. A character has an id, a name, and a freeform markdown body. Everything else the writer cares about goes in the body. This is what separates us from bibisco's empty-form problem.

### Format changes

Two new top-level folders parallel to `manuscript/`, `research/`, `templates/`, `trash/`:

```
MyNovel.chikn/
├── characters/
│   ├── sarah-bennett.md          # freeform body — bio, notes, arc
│   └── sarah-bennett.meta        # structured entity data
├── locations/
│   ├── motel-room-12.md
│   └── motel-room-12.meta
└── ...
```

`.meta` for a character or location:

```yaml
id: 550e8400-e29b-41d4-a716-446655440000
name: Sarah Bennett
type: character              # or "location"
created: 2026-04-23T14:30:00Z
modified: 2026-04-23T14:30:00Z
aliases: [Bennett, Red]      # optional
role: protagonist            # optional hint; free-form string
```

`project.yaml` hierarchy gains top-level folder entries for these (like `manuscript`, `research`, `trash`). Self-healing auto-creates them on open if a `.chikn` is opened in a version that expects them.

### UX

**Binder:** new sections alongside Manuscript/Research/Trash:
- **Characters** — lists character entries; click to open entity editor (markdown body + structured fields)
- **Locations** — ditto for locations

**Entity editor view** (replaces the normal editor pane when an entity is selected):
- Top: inspector-style fields (name, aliases, role) inline
- Body: full TipTap editor over the markdown body (writer's freeform notes)
- Right sidebar: "Appears in" — auto-computed list of scenes that reference this entity

**Scene inspector**: gains live "Characters in this scene" section listing POV + `characters_in_scene` with links that jump to the entity editor.

**Scene-side creation shortcut:** in the POV/characters dropdown, type a name that doesn't exist → "+ Create character 'Name'" — creates the entity on the fly, body initialized empty.

### Implementation

- **Rust core:** new types `Entity` (character/location), new readers/writers under `project/entity.rs`, hierarchy extension to include typed folder entries. Reader walks `characters/` and `locations/` like `manuscript/` / `research/`.
- **Tauri commands:** `list_entities(project_path, type) -> Vec<Entity>`, `create_entity`, `update_entity`, `delete_entity`. Existing `move_node` / `rename_node` work on entities.
- **UI:** new `components/entities/EntityView.tsx`, new section in `Binder.tsx`. Inspector gets a `CharacterEntity` and `LocationEntity` variant.
- **Cross-ref:** `Project` type gains `characters: HashMap<String, Entity>` and `locations: HashMap<String, Entity>`. Reference lookup for scenes stays O(1).

### Scope limits

- **Just two entity types** for v1.2: character, location. No `object`, no `faction`, no `timeline_event`. Adding later won't break anything — the `type` field is extensible.
- **No structured character sub-fields** (no "Age", "Hair color", "Motivation" boxes). Writers who want those use the markdown body.
- **No relationship graphs** between characters. Markdown body or collections can carry that manually. A graph view is a v1.3 question.

### Open questions

- Should entities be automatically git-committed when created (like new scenes are)? Proposal: yes, same rules — debounced save + eventual auto-commit.
- Should an entity be movable to Trash? Proposal: yes, with a confirmation that shows "This character is referenced in 12 scenes — deleting will leave them with 'Unknown' POV." Let the writer decide.

---

## 3. Plot threads

### Why

A novelist tracks multiple simultaneous storylines — A-plot, B-plot, romance, subplot, mystery. Scrivener doesn't do this natively (you use labels and fight it). Manuskript and oStorybook both ship a plot-thread system; it's the single most requested novelist feature not in Scrivener.

The win is *seeing* structure: "give me all the A-plot scenes in manuscript order" or "every romance scene in story-time order" collapses a rereading session into a scroll.

### Format changes

One new top-level file (not a folder — threads are few and have no body):

```yaml
# threads.yaml at the project root
threads:
  - id: main-plot
    name: Main Plot
    color: "#3b82f6"
    description: >
      Sarah uncovers the truth about the motel.
  - id: romance
    name: Sarah & Marcus
    color: "#ef4444"
  - id: mystery
    name: Who killed Daddy?
    color: "#f59e0b"
```

Scenes carry `threads: [...]` in their `.meta` (see Scene Metadata above).

### UX

**Revisions panel gains a "Threads" tab** alongside History and Drafts, showing each thread with:
- Name + color swatch
- Scene count, total word count on this thread
- Click to reveal the full scene list (manuscript order) with synopsis — clicking a scene opens it

**Thread editor** (new modal, triggered by "+ New Thread" or clicking thread name): name, color picker, optional description.

**Scene inspector:** multi-select thread chips, pre-populated with existing threads, "+ New" to create inline.

**Binder:** tiny colored dots next to each scene showing thread membership. 3 threads = 3 dots.

### Implementation

- **Rust core:** new `Thread` struct, `threads.yaml` reader/writer, `Project.threads: Vec<Thread>`. Self-healing: if `threads.yaml` missing but any scene references a thread, create it with default name.
- **Tauri:** `list_threads`, `create_thread`, `update_thread`, `delete_thread`, `threads_scenes(thread_id) -> Vec<Scene>` (the last is a query helper).
- **UI:** new `components/threads/` directory, Threads tab in Revisions panel.
- **Render:** tiny color dots in binder — CSS `::before` pseudo-elements, or inline `<span>` chips.

### Scope limits

- **Flat threads, no hierarchy.** No sub-threads, no act structure. Writers can encode that via naming convention ("Act 2: Romance escalates") if they want.
- **No thread arcs** (beginning/rising/climax markers per scene). That's v1.3.
- **No relationship between threads** (which threads are parallel, which converge). Body text of the thread description can say so prose-style.

### Open questions

- Should threads be deletable if scenes reference them? Proposal: yes, with confirmation; deletion strips the reference from every scene's `.meta`.
- Default colors when writers don't pick one? Proposal: cycle through an 8-color palette in the order threads are created.

---

## Cross-frontend rollout

- **Tauri** ships all three features first (fullest frontend, highest impact).
- **TUI** gets Tier 1 as read-only display in the inspector panel + scene metadata in .meta editing; threads as a flat picker. Full editing optional for v1.2.
- **SwiftUI / Qt6** update `ChiknKit` / core bridge to recognize the new fields — read-only display first, editing in v1.3.
- **Windows (WinUI)** same model — read-only first.

Format spec (`docs/CHIKN_FORMAT_SPEC.md`) gets a **v1.2 section** documenting the new schema. All new fields are optional; v1.1 readers ignore them.
