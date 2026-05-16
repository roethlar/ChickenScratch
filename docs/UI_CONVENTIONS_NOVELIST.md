# Novelist UI Conventions

Key names that novelist-mode UIs agree to store in a document's `fields` map. **This is a UI convention, not a format spec.** The `.chikn` format itself is genre-agnostic — see [`CHIKN_FORMAT_SPEC.md`](CHIKN_FORMAT_SPEC.md). UIs that implement novelist features (scene metadata, POV tracking, plot threads) use the keys below so `.chikn` projects interoperate between Tauri, SwiftUI, Qt6, WinUI, and the TUI without each UI inventing its own naming.

## Scope

- Applies to documents under a writer's manuscript (whatever folder the project treats as manuscript — by default `manuscript/`).
- Does **not** apply to research, templates, or arbitrary files — UIs may attach other `fields` there by other conventions.
- Optional. A novelist-mode UI can implement any subset. Fields a UI doesn't understand must still round-trip untouched (format guarantee).

## Keys

All entries live under the document's `fields` map in its `.meta` sidecar.

### `pov_character`

Type: string. Slug/id of the character whose point of view the scene is written from.

Resolution: free-form until a UI implements character entities. Once the novelist UI ships a `characters/` folder with entities, `pov_character` holds the entity's `id` or its slug filename (either accepted; entity resolution is best-effort).

```yaml
fields:
  pov_character: sarah-bennett
```

### `location`

Type: string. Slug/id of the location the scene takes place at. Same resolution model as `pov_character`.

```yaml
fields:
  location: motel-room-12
```

### `story_time`

Type: string. Time within the story's world, free-form. UIs that try to parse it should accept ISO 8601 (`2024-03-15T22:30`), an ISO date (`2024-03-15`), and prose (`Day 3, 22:30`, `Chapter 2 morning`) — whatever parses goes into the timeline view; unparseable strings are displayed as-is.

```yaml
fields:
  story_time: "Day 3, 22:30"
```

### `duration_minutes`

Type: integer. How many minutes of story-time the scene covers. Used by timeline views to size blocks and by pacing reports.

```yaml
fields:
  duration_minutes: 45
```

### `threads`

Type: list of strings. Plot thread ids the scene advances. Thread definitions (name, color, description) live separately in `threads.yaml` at the project root — a novelist-UI convention file, not part of the format.

```yaml
fields:
  threads:
    - main-plot
    - romance
```

### `characters_in_scene`

Type: list of strings. Character ids beyond the POV character. POV is intentionally separate (it appears once, in `pov_character`) so a query can ask for scenes from Sarah's POV without also surfacing scenes Sarah merely appears in.

```yaml
fields:
  characters_in_scene:
    - marcus-rivera
    - kelly-chen
```

## Example

A full `.meta` for a scene:

```yaml
id: 01234abcd-ef01-...
name: Shelly meets Corn
synopsis: First real encounter at the motel.
status: Draft
keywords: [first-meeting, motel]
word_count_target: 1200
include_in_compile: Yes
fields:
  pov_character: sarah-bennett
  location: motel-room-12
  story_time: "Day 3, 22:30"
  duration_minutes: 45
  threads:
    - main-plot
    - romance
  characters_in_scene:
    - marcus-rivera
```

## Rules for novelist UIs implementing this

1. **Write keys exactly.** Don't add prefixes (no `novelist.pov_character`), don't capitalize, don't translate.
2. **Omit empty values.** Absent key is different from `null`. Writers must skip keys whose value is empty so `.meta` diffs stay quiet.
3. **Preserve unknown entries.** If a user's `.meta` has a `fields` key you don't recognize (added by a different UI or a future version), keep it on write. This is not optional — it's the format contract.
4. **Invalid references are opaque strings, not errors.** `pov_character: sarah-who-does-not-exist` loads the string, displays "Unknown," and offers to clear the reference. It does not fail validation.
5. **Reference validation is scoped to known keys.** The Tauri validator checks only `pov_character`, `characters_in_scene`, `location`, and `threads`. Custom convention keys remain opaque unless a UI publishes and implements its own validator.
6. **Extensions that outgrow `fields`** — e.g., a full character bible — belong in their own files under the project root (e.g., `characters/sarah-bennett.md` + `.meta`), tracked by git, discovered by convention. They are not format-mandated; they are how the novelist UI chooses to organize.

## Related conventions (for other domains)

The same `fields` mechanism is how other domains store their own vocabulary. A TTRPG-campaign UI publishes its own `UI_CONVENTIONS_TTRPG.md` listing `session_date`, `encounter_cr`, etc. A lab-notebook UI publishes its `UI_CONVENTIONS_LAB.md`. No coordination between domains is required; the format guarantees each UI only modifies keys it understands and preserves the rest.

## Reserved?

No. Nothing in `fields` is reserved by the format. If you build a novelist UI that wants a different vocabulary, do it. This document exists so that the five frontends in this repo interoperate — it doesn't gate anyone else.
