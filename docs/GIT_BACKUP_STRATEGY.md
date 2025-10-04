# Git + Snapshot Strategy for .chikn Projects

## Goals

- Keep every `.chikn` project as a first-class folder that users can move, copy, or zip without tooling.
- Give low-tech authors Scrivener-like safety nets: automatic snapshots, easy rollbacks, no Git jargon.
- Unlock rich history features (milestones, draft sandboxes, diffs, collaboration) without exposing Git directly.
- Support both standalone projects and “one repo that tracks all my stories” workflows.

## Layers of Protection

1. **Working tree** – `project.yaml`, `manuscript/`, `research/`, etc. remain the canonical files. Writers always interact with these directly.
2. **Snapshots (`revs/`)** – automatic tarball archives of the working tree (excluding Git internals). Quick, no-thinking restore points for “oops” moments. Retention policy keeps storage small.
3. **Git-backed history** – invisible to the author but available for fine-grained Milestones, sandbox drafts, diffing, and future collaboration/sync features.

## Snapshot Mechanics (`revs/`)

```
MyStory.chikn/
├── project.yaml
├── manuscript/
└── revs/
    ├── 2025-04-10T14-00-00_auto.tar.gz
    ├── 2025-04-03T09-30-00_before-compile.tar.gz
    └── manifest.json
```

- Tarballs capture the entire project folder (minus `.git/`) so a manual extract yields a ready-to-open `.chikn`.
- `manifest.json` tracks timestamps, trigger reason, and user notes.
- Default retention (e.g., last 10 snapshots) keeps disk usage tiny for prose projects.
- `revs/` is ignored by Git (`.gitignore`) so it doesn’t clutter parent repositories.
- In-app restore unpacks the tarball and rebuilds the Git view; advanced users can also extract manually elsewhere.

## Git Integration

### Internal Git as Plumbing

- Every project has a hidden Git repository—either inside the `.chikn` folder or in a parent “umbrella” repo—initialized automatically.
- The app commits after meaningful events (autosave interval, manual Milestone, snapshot, compile) with writer-friendly messages.
- Git remains invisible in the UI; writers see “Revision History,” “Sandbox Draft,” “Merge Sandbox,” etc.
- Branches underpin “sandbox drafts”: creating a sandbox spins up a branch; merging applies Git’s diff/merge engine.

### Handling External Repos

- On project open, walk up the directory tree:
  - If a parent `.git/` exists, use that repo and register the project as a Git worktree or dedicated branch (e.g., `refs/chikn/ProjectName`).
  - If none exists, create/use an internal `.git/` inside the project folder.
- This avoids nested-repo warnings and keeps external workflows intact (one repo for multiple stories).
- The app’s background service schedules and manages Git operations; if the repo corrupts, it self-heals using the latest snapshot.

### Why Pair Git with Snapshots?

- **Snapshots** are fast recoveries (“rewind to 10 minutes ago”) and work even if Git is absent or broken.
- **Git** provides rich history (line-level diffs, metadata), branching, merges, and integration with future cloud sync or collaboration.
- Together they deliver trust: writers get simple restore points, and advanced features tap into Git without exposing it.

## Storage Footprint Examples

For a 2,500-word story with three months of work:
- Working files: ~20 KB.
- Git repo (auto-commits every milestone): a single packfile, typically <500 KB.
- Ten snapshots: each tarball compresses to a few KB, totaling <100 KB.
- Snapshot count and Git garbage collection keep total projects well below a megabyte.

## Implementation Notes

- **Initialize Git silently** when a project opens. If the repository is missing or invalid, recreate it from the working tree.
- **Snapshot service** runs on an interval and on triggers (before compile, manual snapshot). Stores tarballs and updates `manifest.json`.
- **History service** queues commits with friendly messages (e.g., `"Milestone: Draft 1 Complete"`).
- **Sandbox drafts** map to Git branches and detached worktrees. Writers see terms like “Experimental Draft” and “Promote Sandbox,” never “branch.”
- **Restore flow**: Choose snapshot → tarball unpacks → Git reset or rebase to match → snapshot logged.
- **Parent repo integration**: use Git worktrees so each `.chikn` behaves independently while sharing `.git` storage higher up.
- **Corruption fallback**: if Git operations fail, alert the user and offer a one-click restore from the latest snapshot.

## Future Extensions

- Stored history enables comparisons across projects (e.g., build a collection from selected story drafts).
- Git remotes let advanced users sync to GitHub/Gitea; the UI can expose “Publish Revision” later without changing the core design.
- Snapshot retention could adapt to disk space or project size.

## Summary

Users keep working with `.chikn` folders like ordinary files. Under the hood, the application layers:
- Automatic snapshots for simple, resilient recovery.
- Invisible Git history for rich revision features when available.
- Flexible repo detection so the same project works standalone, inside a larger Git workspace, or shared through a zip/USB drive.

This hybrid approach preserves the file-first UX writers expect while giving the product room to grow into full version control and collaboration without sacrificing trust.
