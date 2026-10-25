# Plan: A reload never loses work, and the web version works on a phone

**Owner request (quote):**
> reloading the page should not abandon the open project

> more than that. a reload should not abandon any unsaved changes in the
> currently open doc.

> how do I start a new project once a project has been opened? there's no
> close

> this needs a mobile-friendly UI for taking quick notes or edits on a mobile
> device

(2026-10-25, after the first install of the web version on a second machine;
then "proceed with planning as directed in state docs, then implement all
slices".)

**Phase check:** [x] Allowed — the first two items are I6 (no writer data
loss) gaps in the web shell, the rest is the owner's queued side quest for
the same shell  [x] Not paused

**Invariants touched:** I6 (the browser keeps a safety copy of unsaved text
and puts it back; nothing on disk is overwritten without the writer's say);
I2 untouched (no format I/O outside `crates/core`; the only Rust change is one
lease endpoint in `crates/web`); I7 (nothing git-shaped reaches the UI).

---

## [MODEL] Intent

In the browser, a reload, a crashed tab or a phone that killed the page in
the background brings the writer straight back to the project and note they
had open, with the words they typed in the last seconds still there. A
**Projects** button saves, hands the project back and shows the list, so a
second project (or a second device) is one tap away. On a phone the same
page is usable: one column, the binder slides in from the side, buttons are
finger-sized, a note can be started in one tap from the project list, and the
page can be added to the home screen like an app.

## [MODEL] Approach

### Slice 1 — the safety copy, the remembered place, the Projects button

**Gap today.** Autosave is debounced (`Editor.tsx`: at least 250 ms, 2 s by
default). `App.tsx` flushes on `beforeunload`, but a browser does not wait
for the request, so a reload inside the window loses the last typing. The
store's `closeProject` exists but nothing calls it; after a reload the
Welcome screen appears and the writer re-picks the project by hand.

**Safety copy** (`ui/src/components/editor/safetyCopy.ts`, pure, tested):

- Key `cs-draft:<projectPath>:<docId>` in `localStorage`, value
  `{ content, base, at }`: the buffer's markdown, the disk text that buffer
  was loaded from, and a timestamp.
- Written from the editor's update path at most every 500 ms, and
  synchronously on `pagehide` / `beforeunload` (storage writes are
  synchronous, so the final keystrokes land even when the save request
  cannot). Flow mode splits the buffer at its boundaries and writes one key
  per section.
- Cleared when the save of exactly that text succeeds (the buffer has not
  moved on since). A save that fails, or a reload mid-save, leaves it.
- Restore, in the editor's load effect, per document: no copy → nothing.
  Copy equal to disk → remove it. Copy differs and disk still equals `base`
  → load the copy into the buffer, mark dirty so autosave writes it, one
  info toast: "Restored unsaved changes to <name> from this browser." Copy
  differs and disk moved (another device or tab saved since) → never
  overwrite silently: a yes/no dialog names the note and asks whether to use
  the unsaved text from this browser or keep what is on disk; "keep"
  discards the copy on the writer's say.
- Storage is per browser profile, so a copy made on the phone is restored on
  the phone. Storage that is missing or throws is tolerated (the copy is a
  net, not a requirement).

**Remembered place** (web shell only, `ui/src/lastOpen.ts`): `cs-last-project`
and `cs-last-doc:<projectPath>` in `localStorage`, written when a project
opens and a document is selected, cleared by the Projects button. On first
render with no project, the web shell opens the remembered project and then
selects the remembered note; a failure shows the Welcome error exactly as a
hand-pick does and forgets the entry so a dead project cannot loop. The
session id already lives in `sessionStorage`, so a reload of the same tab
keeps its lease seat; a new tab or device still gets the existing
"Take over" offer.

**Projects button** (`App.tsx`, first in the view toolbar, both shells):
refuses while a barrier operation is running (toast), flushes the pending
save (a failed flush keeps the project open and says so), runs the same
`backup_on_close` the window-close path runs, asks the server to release the
lease (`POST /api/lease/release`: the holder gives the project up at once
instead of after the two-minute idle expiry; the desktop shell skips this),
clears the remembered place, calls `closeProject`. The Welcome screen
refreshes its list on mount as it does today.

### Slice 2 — one column on a narrow screen

Pure layout: `@media (max-width: 768px)` in `editor.css`, one hook
(`useNarrowScreen`, `matchMedia`) in `App.tsx`.

- `.app` becomes a single column; the binder is an off-canvas drawer
  (`.binder-drawer`, slides from the left over a scrim) opened by a Menu
  button at the start of the toolbar and closed by tapping the scrim or
  picking a document (`selectDocument` closes it). The resize handle is not
  rendered on narrow screens.
- Inspector and Revisions open as full-width sheets under the toolbar
  (`position: fixed; top: toolbar; inset 0`) so their existing toolbar
  toggles still close them. Nothing inside those components changes.
- Both toolbars (view and formatting) scroll horizontally instead of
  wrapping; the status line stays one row.
- `100dvh` instead of `100vh` so the browser chrome and the on-screen
  keyboard do not hide the status line; `viewport-fit=cover` and
  `interactive-widget=resizes-content` in `index.html`.
- Welcome: action buttons full width, project rows taller.

### Slice 3 — a note in one tap

On the web Welcome, each project row gains a **Quick note** button (icon,
44 px target on touch). It opens the project, creates a document at the root
named from the date and time ("Note 2026-10-25 14:03"), selects it, and the
editor takes focus. The remembered place is updated so a reload lands on the
new note. The desktop Welcome is unchanged.

### Slice 4 — finger-sized controls and home-screen install

- `@media (pointer: coarse)`: toolbar buttons, binder rows, dialog buttons
  and project rows reach 44 px targets; hover-only reveals (binder actions)
  are always visible.
- `ui/public/manifest.webmanifest` (name, `display: standalone`,
  `start_url: /`, theme colours, icons 256 and 512 copied from
  `src-tauri/icons`), `<link rel="manifest">`, `theme-color`,
  `apple-mobile-web-app-*` metas and `apple-touch-icon` in `index.html`.
  Vite copies `public/` into `dist/`, which both `--ui-dir` and the embedded
  build serve; the embedded MIME table learns `webmanifest`. No service
  worker: the page must always load the server's current UI and never a
  cached one.
- User guide: a "On a phone" subsection under the web version (install from
  the browser's share / menu, Quick note, where the binder went, what the
  safety copy does).

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `ui/src/components/editor/safetyCopy.ts` (+ test) | write / clear / decide-restore helpers over `localStorage` |
| `ui/src/components/editor/Editor.tsx` | throttled + unload writes; restore on load; clear on confirmed save |
| `ui/src/lastOpen.ts`, `ui/src/App.tsx`, `ui/src/stores/projectStore.ts` | remembered project + note; auto-reopen (web); Projects button |
| `crates/web/src/lease.rs`, `app.rs`, `tests/http.rs`; `ui/src/commands/web.ts` | `POST /api/lease/release` |
| `ui/src/editor.css`, `ui/index.html`, `ui/src/hooks/useNarrowScreen.ts`, `App.tsx`, `Binder.tsx` | narrow layout, drawer, sheets, scrolling toolbars |
| `ui/src/components/welcome/Welcome.tsx` (+ test) | Quick note |
| `ui/public/manifest.webmanifest`, icons, `ui/index.html`, `crates/web/src/static_ui.rs` | install metadata; `webmanifest` MIME |
| `docs/USER_GUIDE.md`, `DEVLOG.md`, `docs/plans/README.md`, `.agents/state.md` | records |

## [MODEL] Tests

- [ ] `safetyCopy.test.ts`: write/clear round trip; "equal to disk" removes;
      "disk unchanged" restores; "disk moved" asks; storage that throws is
      tolerated.
- [ ] `editorSafetyCopy.test.tsx`: typing writes a copy; the confirmed save
      clears it; a mount with a copy and unchanged disk shows the copy and
      saves it (and fails with the restore removed — the bite proof).
- [ ] `lastOpen.test.tsx`: the web shell reopens the remembered project and
      note; a failed open forgets it; Projects button flushes, backs up,
      releases, clears, shows Welcome.
- [ ] `cargo test -p chickenscratch-web`: release by the holder frees the
      project for another session; release by a non-holder is a no-op.
- [ ] `welcomeWeb.test.tsx`: Quick note creates and selects a dated note.
- [ ] `useNarrowScreen.test.tsx`: the drawer renders only under the
      threshold and closes on document pick.
- [ ] Declared suite (`.agents/repo-guidance.md`), Tauri crate excepted on
      this box (Known drift); CI on push.

## [MODEL] Owner verification (plain English)

1. In the browser, type a few words and reload at once: the same project and
   note come back with those words in place, and a one-line notice says they
   were restored.
2. Click **Projects** (first button in the toolbar): the project list shows;
   open a different project or create one.
3. On the phone, open the server's address: one column, a menu button opens
   the binder, the formatting bar scrolls sideways, buttons are easy to tap.
   On the project list, tap the note icon beside a project: a new dated note
   opens ready to type.
4. From the browser's menu, "Add to Home Screen": the icon opens the app full
   screen.

## [YOU] Decisions needed

- None blocking. Taken under the go "implement all slices": the remembered
  place is web-only (a desktop launch keeps showing Welcome); the Projects
  button exists in both shells; no service worker (always the live UI).

## Status

- 2026-10-25: drafted and all slices approved in one go (owner: "proceed
  with planning as directed in state docs, then implement all slices").
