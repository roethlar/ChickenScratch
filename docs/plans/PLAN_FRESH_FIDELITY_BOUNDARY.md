# Plan: Fresh-fidelity operation boundary

**Status:** Shipped 2026-07-12 in `a0e7621`; guard proofs and verification are
recorded in `DEVLOG.md`.

**Owner request (quote):**
> yes

Approval context presented immediately before the owner's response: after a
project opens, outside file changes do not invalidate ChickenScratch's cached
write permission; recheck safety before each logical mutation and make normal
reads non-writing unless repair is authorized, at the cost of one additional
disk scan per save.

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (Engine hardening Step 2)
[x] Not paused

**Invariants touched:** I1 (the project folder is the durable product), I2
(the core engine owns project reads/writes), I5 (preserve externally written
metadata), I6 (no writer data loss), I9 (guard proofs + declared suite).

---

## [MODEL] Intent

An engine session may retain a root/epoch-bound `WriteToken`, but every
top-level mutation must obtain a short-lived operation permit from a fresh,
side-effect-free fidelity probe. If another tool changes a formerly Full
project into a Degraded shape after it opens, the next save, revision, folder
creation, deletion, or git operation refuses before touching bytes. Ordinary
reads and exports never create folders or quarantine files. Missing standard
folders on a still-Full project retain their existing benign self-heal, but
only through an explicitly permitted initial-open path.

## [MODEL] Evidence and constraints

- `WriteToken` currently proves fidelity only when issued. Its epoch observes
  engine tree replacements, not external filesystem changes.
- Tauri caches non-stale tokens for the session. A later external change to a
  newer `format_version` or corrupt sidecar therefore does not invalidate the
  cached token.
- Public `reader::read_project` currently uses `RepairMode::SelfHeal` without
  a token: it creates standard folders and quarantine-renames corrupt
  sidecars. CLI `.chikn` export calls this public reader directly.
- Re-probing inside every leaf `ensure_valid_for` call is incorrect. Composite
  operations deliberately pass through an intermediate inconsistent tree:
  folder deletion removes several files before writing the smaller manifest;
  document/revision restore replaces bytes before committing the forward
  restore. A probe between those internal steps would abort halfway.
- The correct boundary is one fresh probe per logical operation, followed by
  cheap root/epoch checks for every nested engine mutation under that permit.

## [MODEL] Approach

1. **Split session authority from operation authority.** Keep `WriteToken` as
   the non-Clone, root-bound, epoch-stamped session capability. Add a
   non-Clone, privately constructible `WritePermit` that can exist only after
   `WriteToken::with_write_permit(project_path, operation)` performs, in
   order: canonical-root equality, epoch freshness, a side-effect-free
   `probe_project_fidelity`, and a second epoch check. Only `Fidelity::Full`
   enters the operation closure. The closure prevents the permit from being
   cached as application state or escaping the logical operation.
2. **Require the permit structurally.** Existing-project mutators accept
   `&WritePermit`, never bare `&WriteToken`: project write/delete/app-file/
   subdirectory APIs, composite deletion, every token-gated git API, and the
   explicit repairing read. Permit leaf checks verify canonical root and
   expected epoch only; they never repeat the full probe. `create_project`
   remains the documented pre-token bootstrap exception because it refuses
   an existing path. The separately recorded public `init_repo` bootstrap
   hole is not broadened or silently declared fixed by this slice.
3. **Reuse one permit across composites.** Core deletion and Tauri recursive
   deletion reuse the same permit across all child file removals and the
   final `write_project`. Restore and backup paths reuse the permit through
   their internal revision commit rather than starting a nested probe.
   Network pull may revalidate fidelity after fetch and immediately before
   its first worktree mutation because fetch itself does not alter project
   content. Do not change restore product semantics or introduce a generic
   recovery bypass in this slice.
4. **Make default reads pure.** Change public `read_project` to the current
   repairs-disabled behavior: in-memory reconciliation remains, but no folder
   creation and no quarantine rename occurs. Retain `read_project_readonly`
   as a source-compatible pure alias for now. Add
   `read_project_with_repair(path, &WritePermit)` for Full-project initial
   opens; it may create missing standard folders through existing safe-path
   code, then reads. Corrupt sidecars are always preserved in place and
   treated as missing in memory—quarantine is not an automatic read action.
5. **Move frontend operation boundaries.** Tauri `ProjectTokens` keeps cached
   `WriteToken`s but exposes a closure-based fresh-permit operation; a permit
   failure invalidates the cache and returns the plain `ReadOnly` reason.
   Every mutating Tauri command opens one permit inside its existing
   per-project lock and uses it for the complete read/modify/write or git
   operation. Tauri and TUI Full initial opens use explicit permitted repair;
   Degraded opens, pure queries, compile, search, and CLI export use the pure
   reader. TUI saves/revisions and converter import likewise open one permit
   per logical operation.
6. **Keep the guarantee precise.** This reduces the external-change window
   from the full application session to one operation. It does not make a
   probe plus multiple filesystem writes atomic against an arbitrary external
   process; per-file safe-path validation and atomic replacement remain the
   final defenses. Cross-process locking and full project transactions are
   separate hardening work.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/src/core/project/fidelity.rs` | operation-scoped `WritePermit`; fresh probe at permit issuance; root/epoch leaf checks |
| `crates/core/src/core/project/reader.rs` | pure default reader; explicit permit-backed folder repair; remove automatic quarantine mutation |
| `crates/core/src/core/project/writer.rs` | accept `WritePermit` for existing-project writes/deletes/app files/subdirs |
| `crates/core/src/core/project/deletion.rs` | reuse one permit across recursive deletion |
| `crates/core/src/core/git.rs` | accept/reuse one permit across git mutations and internal composite commits |
| `src-tauri/src/commands/mod.rs` | cached-token to fresh-permit operation boundary and cache invalidation |
| `src-tauri/src/commands/{project,document,git,io,templates,threads}.rs` | one permit per locked logical mutation; explicit repair only on Full initial open |
| `crates/tui/src/{app,main}.rs` | permit-backed Full open and each save/revision operation |
| `crates/cli/src/scrivener/converter/mod.rs` | one permit across the existing import operation |
| `crates/core/src/**` and `crates/core/tests/**` | mechanical call-site migration plus guard tests |
| `crates/cli/src/main.rs` tests | prove `.chikn` export never mutates a corrupt source project |
| `docs/plans/PLAN_FRESH_FIDELITY_BOUNDARY.md`, `DEVLOG.md`, `.agents/state.md` | execution status, guard proof, verification, and close-out |

## [MODEL] Tests

- [x] Public/default read of a corrupt-sidecar + missing-folder fixture returns
  a browsable in-memory project while the source tree stays byte-identical;
  corrupt bytes remain at the original path and no `.corrupt-*` appears.
- [x] Full project -> acquire session token -> externally change
  `format_version` to `9.9` (and separately corrupt a sidecar) -> fresh permit
  issuance returns `ReadOnly`, keeps the old in-process epoch distinguishable,
  invalidates the Tauri cached token, and changes no bytes.
- [x] Representative mutation families cannot be invoked after external
  degradation: project write, document delete, app-file write, project-subdir
  creation, and revision save. Their shared permit gate covers the remaining
  same-family entry points without network-heavy duplicate tests.
- [x] One permit successfully deletes a folder containing at least two
  documents and writes the final manifest; the result probes Full and neither
  document resurrects. This prevents accidental nested re-probing.
- [x] One permit successfully completes restore -> forward revision commit;
  internal continuation must not request a second fidelity probe after the
  intentional tree replacement.
- [x] Full projects missing benign standard folders still repair only through
  `read_project_with_repair`; the Corn sample remains Full, opens normally,
  and writes.
- [x] CLI `.chikn` export given a corrupt-sidecar source and a deliberately
  failing output path errors without changing the source tree or creating a
  quarantine file.
- [x] Retain existing cross-root and epoch-stale token tests, adapted to the
  permit boundary.

Targeted verification:

```bash
cargo test --locked -p chickenscratch-core --test write_guard fresh_fidelity -- --nocapture
cargo test --locked -p chickenscratch-core --lib fresh_fidelity -- --nocapture
cargo test --locked -p chickenscratch --bin chickenscratch fresh_fidelity -- --nocapture
cargo test --locked -p chikn-converter --bin chikn-converter chikn_export_does_not_mutate_corrupt_source -- --nocapture
cargo test --locked -p chickenscratch-core --test write_guard public_read_of_corrupt_sidecar_and_missing_folders_is_browsable_and_pure -- --nocapture
cargo test --locked -p chickenscratch-core --lib operation_permit_deletes_multiple_documents_and_prevents_resurrection -- --nocapture
cargo test --locked -p chickenscratch-core --test remote_sync restore_revision_clean_worktree_restores_and_commits_forward -- --nocapture
```

Guard proofs (temporary local reversions, never committed):

1. Route public `read_project` back through the mutating repair/quarantine
   mode: public-read and CLI byte-identity guards must fail; restore -> pass.
2. Bypass the fresh fidelity probe at permit issuance / restore cached-token
   reuse: old-session and Tauri-cache guards must fail; restore -> pass.
3. Replace shared-permit recursive deletion with nested fresh permits: the
   two-document deletion guard must fail after the first deletion; restore ->
   pass.

Declared suite before the code commit:

```bash
cargo fmt --all -- --check
cargo clippy --locked -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --all-targets -- -D warnings
cargo test --locked -p chickenscratch-core -p chickenscratch -p chickenscratch-tui -p chikn-converter --lib --bins --tests
scripts/check-release-metadata.sh
cd ui && npm run lint && npm run build
```

## [MODEL] Owner verification (plain English)

1. Open and save a normal current project: it behaves normally.
2. Open an older/incompatible project: it remains read-only and unchanged.
3. Exporting an incompatible project may fail or use its in-memory fallback,
   but the source folder never gains repair or quarantine files.

## [MODEL] Non-goals / parked findings

- Full multi-file transaction journal or crash recovery.
- Cross-process filesystem locking or elimination of the final probe/write
  race against arbitrary external tools.
- Scrivener binary-asset copy boundary, public `init_repo`, revision staging
  of recovery artifacts, `include_in_compile` case tolerance, vault/remotes,
  or project-level `fields`.
- A generic recovery permit or redesign of conflict/force-pull product
  semantics; record any newly exposed recovery blocker rather than widening
  authority inside this slice.

## [YOU] Decisions needed

- None. The owner approved the scope and performance cost on 2026-07-12.
