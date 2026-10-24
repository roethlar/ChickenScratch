# Plan: Project profiles — fiction vocabulary becomes one option among several

**Status:** Queued 2026-10-24 as the next side-quest slice after the web shell.
Owner wording: "plan it" (to the proposal below). Owner 2026-10-24: "complete all slices and push when done" — the go for all
four slices and the push. Progress: slices 1 (profile table), 2 (profile applied to binder, views,
inspector, revisions, statistics, preview) and 3 (New Project asks for the
type) landed 2026-10-24.

**Owner request (quote):**
> this highlights other UI work that needs doing. Characters and Locations is
> fiction specific. as a general note taking / markdown editor, those aren't
> necessarily ideal. how can we generalize? can we have project types that
> include or exclude or just rename these parts while keeping the underlying
> tech stack in tact?

> plan it

**Phase check:** [ ] Not part of the active phase (Engine hardening); queued
by the owner as a side quest, phase unchanged. [x] Not paused: "novelist
feature expansion" is a later priority, and this is the opposite — making the
novelist vocabulary optional. UI-layer only; no engine or format work.

**Invariants touched:** I4 (domain vocabulary stays in UI conventions; the
stored `project_type` already exists in `ProjectMeta` and nothing new is
added to the schema), I5 (display renames never touch stored names), I3/
ADR-007 (lands once in `ui/`, both shells get it). Not touched: I1, I2
(no format I/O changes; the only backend change is more entries in the
app-level template list in `chickenscratch-services`), I6, I7.

---

## [MODEL] Intent

A project declares what kind of thing it is (Novel, Notes, Document, …) and
the app shows the vocabulary and panels that fit: a Notes project has no
Characters or Locations sections, no Corkboard or Timeline, its top folder is
called "Notes", and New from Template offers meeting notes and how-tos
instead of scenes and character sheets. A Novel project looks exactly as
today. The folders on disk keep their current names (`manuscript/`,
`research/`, `trash/`, `characters/`, `locations/`), so a project is readable
by every tool and by every profile, and changing a project's type later only
hides or shows things — nothing is moved or deleted. New Project asks for the
type; the web shell defaults to Notes.

## [MODEL] Evidence and constraints

Verified against `a054e9d` (2026-10-24).

- **The format is already neutral.** `docs/INVARIANTS.md` I4; `ProjectMeta`
  carries `project_type: Option<String>` (`ui/src/types.ts:47`), written
  through `update_project_metadata` (`ui/src/commands/project.ts`). The
  Preview's Edit Details form offers Novel, Short Story, Novella,
  Screenplay, Essay, Document (`ui/src/components/preview/Preview.tsx:179`)
  and uses the type only to decide section titles
  (`Preview.tsx:145-146`: Novel/Novella/Screenplay). `characters/` and
  `locations/` are "optional novelist convention" folders to the engine
  (`crates/core/src/core/project/format.rs:197-203`).
- **Where the fiction vocabulary lives in the UI:**
  - `ui/src/components/binder/Binder.tsx`: entity sections labelled
    "Characters"/"Locations" (~818-826, `EntitySection` ~986-1060);
    standard folders are recognised **by stored name**
    (`node.name === "Manuscript"` etc., ~259, ~299, ~1246-1250, ~1362) — the
    stored name must stay; only the rendered label changes.
  - `ui/src/App.tsx`: view buttons Editor / Corkboard / Manuscript Preview /
    Timeline (~357-380), Statistics, Comments, Export; `view` state persisted
    in `localStorage("cs-view")` may hold a view the profile hides.
  - `ui/src/components/inspector/Inspector.tsx`: Scene section (POV,
    location, story time, duration, threads, characters in scene; novelist
    keys at ~25-30) and `LABEL_PRESETS` (~17).
  - `ui/src/components/revisions/Revisions.tsx`: Threads tab;
    `ui/src/components/stats/StatsPanel.tsx`: session targets;
    `ui/src/components/timeline/TimelineView.tsx`,
    `ui/src/components/corkboard/Corkboard.tsx`: fiction views;
    `ui/src/components/command-palette/CommandPalette.tsx`: actions list.
  - Templates come from the backend (`crates/services/src/templates.rs`
    `default_templates`: scene, chapter, character, setting, outline) and
    are offered by the binder's "New from Template".
- **Existing projects** mostly have an empty or fiction type. Unknown or
  empty must keep today's full fiction UI so nobody loses a panel on upgrade.
- **Both shells** render the same `ui/`; the web shell's New Project is a
  name prompt (`Welcome.tsx` `handleCreateWeb`); the desktop one is a save
  dialog (`handleCreate`). Creating sets no type today.

## [MODEL] Approach

UI-only, one concern per commit. Order 1 → 2 → 3 → 4.

### Slice 1 — The profile table and hook (no visible change)

`ui/src/profiles.ts`:

```ts
export type ProfileId = "fiction" | "notes" | "document";
export interface Profile {
  id: ProfileId;
  label: string;                       // "Fiction", "Notes", "Document"
  features: {
    entities: boolean;                 // Characters/Locations sections, entity creation
    corkboard: boolean;
    timeline: boolean;
    threads: boolean;                  // Threads tab + inspector thread chips
    sceneFields: boolean;              // inspector Scene section
    sessionTargets: boolean;           // Stats panel targets + badge
    previewSectionTitles: boolean;     // Preview: type-aware section headers
  };
  folderLabels: Record<"Manuscript" | "Research" | "Trash", string>;
  templateIds: string[];               // which backend templates to offer
  labelPresets: string[];              // inspector Label presets
  typeOptions: string[];               // project_type values that map here
}
export const PROFILES: Record<ProfileId, Profile>
export function profileFor(projectType: string | null | undefined): Profile
export function useProfile(): Profile   // zustand selector over project.metadata.project_type
export function folderLabel(storedName: string, profile: Profile): string
```

Mapping: `Novel | Novella | Short Story | Screenplay | "" | unknown` →
`fiction` (today's UI, all features on; labels Manuscript/Research/Trash;
templates scene, chapter, character, setting, outline; presets Scene,
Chapter, Outline, Notes, Research). `Notes` → `notes` (all fiction features
off; previewSectionTitles off; labels Notes / Attachments / Trash; templates
meeting, decision, howto, daily; presets Note, Meeting, Idea, Reference,
Todo). `Essay | Document` → `document` (fiction features off,
previewSectionTitles on; labels Document / Research / Trash; templates
outline, howto, decision; presets Section, Draft, Reference, Appendix).
Unit tests for the mapping and for the folder label fallback (a stored name
not in the table is shown unchanged).

### Slice 2 — Apply the profile

- **Binder**: render `folderLabel(node.name)` for the three standard folders
  (stored names untouched everywhere else); render the entity sections and
  the "+ character/location" affordances only when `features.entities`;
  filter the template menu by `templateIds` (unknown ids still shown if the
  backend returns them? no — hidden; the backend list is the superset).
- **App**: hide Corkboard and Timeline buttons when their feature is off; if
  the persisted `view` is hidden by the current profile, fall back to
  `editor` on project open; the Export button and Preview stay for every
  profile (notes export to .docx is useful); Preview heading uses
  `previewSectionTitles` instead of the hard-coded list.
- **Inspector**: Scene section only when `features.sceneFields`; thread chips
  only when `features.threads`; `LABEL_PRESETS` from the profile.
- **Revisions**: Threads tab only when `features.threads`.
- **Stats**: session-target controls and badge only when
  `features.sessionTargets`.
- **Command palette**: omit actions for hidden views.
- **Preview Edit Details**: type select gains "Notes"; options grouped by
  profile with a one-line hint of what changes ("Hides Characters, Locations,
  Corkboard and Timeline; calls the top folder Notes").
- Tests (vitest): Binder in a Notes project shows "Notes" as the top folder
  label, no Characters section, and only notes templates; in a Novel project
  everything is as today (regression guard); App hides Corkboard/Timeline and
  falls back from a persisted hidden view; Inspector hides the Scene section
  for Notes.

### Slice 3 — New Project asks for the type

- Shared `NewProjectDialog` (`ui/src/components/welcome/NewProjectDialog.tsx`)
  replacing the bare name prompt in the web shell and following the save
  dialog on the desktop: name (web only), type radio (Notes / Fiction /
  Document) with the one-line hint per type. Default: Notes in the web shell,
  Fiction on the desktop (today's behaviour).
- After `createProject` succeeds, call `updateProjectMetadata` with the chosen
  `projectType` (two existing commands; no backend change). Also sets the
  type when importing a Markdown folder.
- Tests: web Welcome creates with the chosen type; desktop path still works
  with the default.

### Slice 4 — Notes and document templates (services)

`crates/services/src/templates.rs` `default_templates` gains: `meeting`
("Meeting notes": date, attendees, agenda, notes, actions), `decision`
("Decision record": context, options, decision, consequences), `howto`
("How-to": goal, steps, verification), `daily` ("Daily log": date heading).
Pure data; one unit test asserting the ids exist and bodies are non-empty.
The Tauri and web shells need no change (same command).

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `ui/src/profiles.ts` (new, slice 1), `ui/src/test/profiles.test.ts` (new) | profile table, mapping, hook, folder labels |
| `ui/src/components/binder/Binder.tsx` | folder labels, entity sections gated, template filter |
| `ui/src/App.tsx`, `ui/src/components/command-palette/CommandPalette.tsx` | view buttons and actions gated; hidden-view fallback |
| `ui/src/components/inspector/Inspector.tsx`, `ui/src/components/revisions/Revisions.tsx`, `ui/src/components/stats/StatsPanel.tsx` | scene fields, threads, session targets gated; label presets from profile |
| `ui/src/components/preview/Preview.tsx` | "Notes" type option with hints; section titles from profile |
| `ui/src/components/welcome/NewProjectDialog.tsx` (new, slice 3), `Welcome.tsx` | type choice on create; metadata update after create |
| `ui/src/test/*.test.tsx` | binder/app/inspector/welcome coverage per slice |
| `crates/services/src/templates.rs` (slice 4) | notes/document templates |
| `docs/UI_CONVENTIONS_NOVELIST.md`, `docs/USER_GUIDE.md`, `TODO.md` | record that novelist conventions are the Fiction profile; "Project types" section; feature record |
| `docs/plans/PLAN_PROJECT_PROFILES.md`, `.agents/state.md` | status per slice |

## [MODEL] Tests

- [ ] Slice 1: mapping (`Novel`→fiction, `Notes`→notes, `Essay`→document,
  `""`/unknown→fiction); `folderLabel` falls back to the stored name.
- [ ] Slice 2: Notes project hides Characters/Locations, Corkboard, Timeline,
  Threads tab, Scene section, session targets, and shows "Notes" as the top
  folder; Novel project renders exactly as today (guard); persisted hidden
  view falls back to editor.
- [ ] Slice 3: web New Project creates then sets `projectType: "Notes"` by
  default; choosing Fiction sets `"Novel"`.
- [ ] Slice 4: template ids present; existing fiction templates unchanged.
- [ ] Every slice: `cd ui && npm run lint && npm run build && npm run test`;
  slice 4 also the Rust declared suite.

## [MODEL] Owner verification (plain English)

1. Open the web version, click New Project, keep "Notes", name it "Journal".
   The left column's top folder says Notes; there is no Characters or
   Locations section and no Corkboard or Timeline button.
2. Right-click in the binder → New from Template: Meeting notes, Decision
   record, How-to, Daily log.
3. Open an existing novel: everything is where it was.
4. In the Journal project, open Preview → Edit Details and switch the type
   to Novel: Characters, Locations, Corkboard and Timeline appear; switch
   back and they go away. The files on disk did not change.

## [YOU] Decisions needed

None block the plan. Defaults taken, say so if you want different ones:

- **Empty or unknown type shows the full fiction UI** (today's behaviour), so
  no existing project loses a panel.
- **Web New Project defaults to Notes; desktop defaults to Fiction.**
- **Display-only renames.** Folders on disk keep their names so files stay
  compatible with every profile and tool. If you would rather have a Notes
  project create a folder literally named `notes/`, that is a format
  question for a separate plan.
