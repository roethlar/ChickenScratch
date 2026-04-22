# Revision & Backup Strategy for .chikn Projects

**Status:** Describes the shipping implementation in v0.1.0-alpha. An earlier draft of this document proposed pairing git with `revs/` tarball snapshots; the implementation went git-only, and this rewrite reflects what the code actually does.

## Goals

- Keep every `.chikn` project as a first-class folder that users can move, copy, or zip without tooling.
- Give low-tech authors Scrivener-like safety nets: automatic checkpoints, easy rollbacks, no git jargon.
- Enable rich history features (named revisions, draft versions, diffs) without exposing git directly.
- Support off-site backup via any directory — local, cloud-synced folder, or a git remote.

## How it Works

Each `.chikn` project is a regular folder containing markdown documents and YAML metadata. Revision history lives in `.git/` inside the project. `git2-rs` is linked into the app, so writers never need a system git install.

The UI speaks in writer terms — "Save Revision", "Draft Version", "Restore", "Backup" — and never exposes commits, branches, remotes, or refs.

### Save Revision

Manual checkpoint. Writer types a short description; the app commits everything with that message. Triggers:
- **Manual:** writer clicks Save Revision / Ctrl+R.
- **Auto-commit:** every 10 minutes of active work, if anything changed, the app records an automatic revision (`Auto: {timestamp}`) so no more than ten minutes of work can be lost.

Auto-commits are distinct from named revisions in the history view so writers can scan for meaningful milestones.

### Restore

"Restore to this revision" is non-destructive: the target state is written back into the working tree and recorded as a *new* revision. The prior state is still in history. There is no destructive rewind.

### Draft Versions

Draft versions are git branches behind the scenes. Writers see a name ("alternate ending") and a button to switch between them. "Merge Draft Version" collapses a draft into the main manuscript using git's merge engine; conflicts are surfaced as a writer-facing dialog, not a three-way merge tool.

### Word-level Diff

The revision diff viewer shows word-level tracked changes (insertions in green, deletions in red) per document. This is generated from `git diff` output but post-processed so it reads like Word's Track Changes rather than a source-code diff — no `+`/`-` gutters, no unified-diff hunks, no line numbers.

### Backup

Backup is a second directory (local, cloud-synced, or a git remote URL) that the project is mirrored to. Triggers:
- **On named revision** — every manual Save Revision also pushes to the backup destination.
- **On project close** — configurable in Settings > Backup (default on).
- **Periodic** — configurable interval.

Backup failures are non-fatal; they surface as a toast and are retried next trigger. The backup itself is a plain git push when the destination is a remote URL, or a mirrored `.git/` clone when the destination is a directory — either way, full revision history is preserved.

**Recommended:** point the backup directory at a cloud-synced folder (Dropbox, iCloud Drive, Google Drive) for automatic off-site backup with full history. Advanced users can use a self-hosted Gitea or a GitHub repository instead.

## Self-Healing

On open, the project reader reconciles the hierarchy in `project.yaml` against what's on disk:
- Documents present on disk but missing from hierarchy → added.
- Documents in hierarchy but missing from disk → removed from hierarchy.
- Required top-level folders (`Manuscript`, `Research`, `Trash`) missing → created.

This handles the case where a writer restores a single file from the git CLI, or where a sync tool dropped or duplicated files. The project always opens into a consistent state.

## Storage Footprint

For a 50,000-word novel with six months of daily work:
- Working tree: ~300 KB (markdown is tiny).
- `.git/` with auto-commit every ~10 min: typically 2–5 MB after `git gc`.
- Backup mirror: same order of magnitude.

Total well under 10 MB for a full novel's revision history. No snapshot retention policy is needed.

## Why No Tarball Snapshots

An earlier design paired git with periodic `revs/*.tar.gz` snapshots for redundancy. We dropped it:
- Git already gives us content-addressed, compressed, deduplicated history. Tarballs duplicate that work.
- A separate restore path ("try git; if that fails, unpack a tarball") doubles the surface area for bugs in the most sensitive code in the app.
- Cloud-synced backup plus git's own integrity checks cover the failure mode tarballs were meant to address.

If git ever corrupts, the app rebuilds `.git/` from the working tree and pulls history back from the backup mirror. That's the recovery path, not tarballs.

## Summary

Writers keep working with `.chikn` folders like ordinary files. Under the hood:
- Embedded git (`git2-rs`) gives rich history without requiring a system git install.
- Auto-commit every 10 minutes is the "oops" safety net.
- Named revisions are meaningful milestones with writer-friendly messages.
- Backup is git push (or directory mirror) on named revision, close, and interval.
- Self-healing keeps the project consistent through direct-filesystem edits and sync-tool hiccups.
