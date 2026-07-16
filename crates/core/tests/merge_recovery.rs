//! Merge completion/recovery integration tests
//! (PLAN_TREE_REPLACE_EPOCH_GUARD.md, slice 4).
//!
//! Two live bugs anchor this file: with a merge in progress, (1) a
//! conflict touching `project.yaml` made the fidelity probe error — so no
//! ordinary permit could be issued and the promised Abort was unreachable
//! exactly when the conflict hit a format file; (2) the conflict dialog's
//! force exit routed to `sync_pull_force`, whose own dirty-worktree check
//! fires on every conflicted tree, so it had never worked from a real
//! conflict. The fix: a narrow recovery authority
//! (`acquire_recovery_permit`) issued only while merge state is attested,
//! bound to the specific merge (`MERGE_HEAD` OID + worktree fingerprint),
//! authorizing only complete/abort/force-resolve — plus a blanket
//! `save_revision` refusal mid-merge so no automatic writer can bake
//! conflict markers into history.
//!
//! GUARD-PROOF DRILLS (recorded in DEVLOG): reverting the `save_revision`
//! merge-state refusal lets conflict markers be committed wholesale;
//! moving the restore preflight after the disk write fails the
//! zero-mutation assertions; weakening the fingerprint to status-bits-only
//! misses external content edits between confirmation and the hard reset.

use chickenscratch_core::core::git;
use chickenscratch_core::core::project::fidelity::{
    acquire_recovery_permit, acquire_write_token, probe_project_fidelity, Fidelity, WriteToken,
};
use chickenscratch_core::core::project::reader::{read_project, read_project_recovery};
use chickenscratch_core::ChiknError;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const BASE_DOC: &str = "# One\n\nBase.\n";
const DRAFT_DOC: &str = "# One\n\nDraft version.\n";
const MASTER_DOC: &str = "# One\n\nMaster version.\n";

fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("project.yaml"),
        format!(
            "format_version: '1.2'\nid: prj\nname: {name}\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\nhierarchy:\n- type: Document\n  id: doc-one\n  name: One\n  path: manuscript/one.md\n"
        ),
    )
    .unwrap();
}

fn tk(path: &Path) -> WriteToken {
    acquire_write_token(path).expect("write token")
}

macro_rules! with_permit {
    ($path:expr, |$permit:ident| $operation:expr) => {{
        let operation_path: &Path = $path;
        let token = tk(operation_path);
        token.with_write_permit(operation_path, |$permit| $operation)
    }};
}

/// Healthy single-document project with an initial revision.
fn init_project(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().join("Novel.chikn");
    fs::create_dir_all(root.join("manuscript")).unwrap();
    git::init_repo(&root).expect("init repo");
    write_manifest(&root, "Test");
    fs::write(
        root.join("manuscript/one.meta"),
        "id: doc-one\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\nsynopsis: base\n",
    )
    .unwrap();
    fs::write(root.join("manuscript/one.md"), BASE_DOC).unwrap();
    with_permit!(&root, |permit| git::save_revision(&root, "Initial", permit))
        .expect("initial revision");
    root
}

fn current_branch(root: &Path) -> String {
    git2::Repository::open(root)
        .unwrap()
        .head()
        .unwrap()
        .shorthand()
        .unwrap()
        .to_string()
}

/// Diverge `edit` on a draft and on the original branch, then merge the
/// draft back — producing a real in-progress merge with index conflicts.
fn make_conflict(root: &Path, edit: impl Fn(&Path, &str)) -> Vec<String> {
    let original = current_branch(root);
    with_permit!(root, |permit| git::create_draft(root, "alt", permit)).expect("create draft");
    edit(root, "draft");
    with_permit!(root, |permit| git::save_revision(
        root,
        "Draft edit",
        permit
    ))
    .expect("draft revision");
    with_permit!(root, |permit| git::switch_draft(root, &original, permit)).expect("switch back");
    edit(root, "master");
    with_permit!(root, |permit| git::save_revision(
        root,
        "Master edit",
        permit
    ))
    .expect("master revision");
    match with_permit!(root, |permit| git::merge_draft(root, "alt", permit))
        .expect("merge must run")
    {
        git::MergeResult::Conflicts { files } => files,
        other => panic!("fixture must conflict, got {other:?}"),
    }
}

/// Merge conflict confined to a document body — the project still probes
/// Full, so ordinary tokens remain obtainable.
fn make_doc_conflict(root: &Path) -> Vec<String> {
    make_conflict(root, |root, side| {
        let content = if side == "draft" {
            DRAFT_DOC
        } else {
            MASTER_DOC
        };
        fs::write(root.join("manuscript/one.md"), content).unwrap();
    })
}

/// Merge conflict inside `project.yaml`: conflict markers make the
/// worktree copy unparsable, so the fidelity probe ERRORS — the case that
/// stranded writers before the recovery authority existed.
fn make_yaml_conflict(root: &Path) {
    make_conflict(root, |root, side| {
        let name = if side == "draft" {
            "Draft Name"
        } else {
            "Master Name"
        };
        write_manifest(root, name);
    });
}

/// Merge conflict inside a document sidecar: markers corrupt the `.meta`,
/// so the project probes Degraded.
fn make_meta_conflict(root: &Path) {
    make_conflict(root, |root, side| {
        fs::write(
            root.join("manuscript/one.meta"),
            format!(
                "id: doc-one\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\nsynopsis: {side}\n"
            ),
        )
        .unwrap();
    });
}

/// Simulate the damage today's code could leave behind: a mid-merge
/// commit staged everything (clearing the index conflicts) and committed
/// single-parent, leaving `MERGE_HEAD` lingering over a clean tree.
fn make_lingering_merge_head(root: &Path) {
    make_doc_conflict(root);
    let repo = git2::Repository::open(root).unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("Test", "test@test.local"))
        .unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "old-style mid-merge commit",
        &tree,
        &[&head],
    )
    .unwrap();
    assert!(
        repo.find_reference("MERGE_HEAD").is_ok(),
        "fixture must keep MERGE_HEAD lingering"
    );
}

fn head_id(root: &Path) -> git2::Oid {
    git2::Repository::open(root)
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap()
}

fn head_parent_ids(root: &Path) -> Vec<git2::Oid> {
    let repo = git2::Repository::open(root).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    head.parent_ids().collect()
}

fn merge_head_id(root: &Path) -> Option<git2::Oid> {
    git2::Repository::open(root)
        .unwrap()
        .find_reference("MERGE_HEAD")
        .ok()
        .and_then(|r| r.target())
}

/// Byte-exact snapshot of every file under `root` (excluding `.git`, whose
/// internals legitimately change on reads like status refreshes).
fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if rel == ".git" || rel.starts_with(".git/") {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                visit(root, &path, out);
            } else {
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

fn assert_read_only_mentions<T: std::fmt::Debug>(result: Result<T, ChiknError>, fragment: &str) {
    match result {
        Err(ChiknError::ReadOnly(message)) => assert!(
            message.contains(fragment),
            "expected refusal mentioning {fragment:?}, got {message:?}"
        ),
        other => panic!("expected ReadOnly refusal mentioning {fragment:?}, got {other:?}"),
    }
}

// ── Merge-state query ────────────────────────────────────────────────────

#[test]
fn merge_state_reports_conflicts_and_answers_without_any_permit() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    let state = git::merge_state(&root).unwrap();
    assert!(!state.in_progress);
    assert!(state.conflicted_files.is_empty());

    make_doc_conflict(&root);
    let state = git::merge_state(&root).unwrap();
    assert!(state.in_progress);
    assert_eq!(state.conflicted_files, vec!["manuscript/one.md"]);
}

#[test]
fn merge_state_answers_with_conflicted_project_yaml() {
    // The persistent banner keys on this query, and it must answer in the
    // exact situation the fidelity probe cannot (format-file conflicts).
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_yaml_conflict(&root);
    assert!(probe_project_fidelity(&root).is_err());
    let state = git::merge_state(&root).unwrap();
    assert!(state.in_progress);
    assert!(state.conflicted_files.iter().any(|f| f == "project.yaml"));
}

// ── save_revision blanket refusal (rounds 6–9) ──────────────────────────

#[test]
fn save_revision_refuses_during_conflicted_merge_for_every_caller() {
    // DRILL: revert the merge-state refusal in save_revision and this
    // commits the conflict markers wholesale (add_all clears conflict
    // entries unconditionally) — head moves, MERGE_HEAD lingers.
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let head_before = head_id(&root);

    assert_read_only_mentions(
        with_permit!(&root, |permit| git::save_revision(
            &root,
            "Auto: must refuse",
            permit
        )),
        "merge is in progress",
    );

    assert_eq!(head_id(&root), head_before, "no commit may be minted");
    assert!(merge_head_id(&root).is_some(), "merge state preserved");
    assert!(
        git2::Repository::open(&root)
            .unwrap()
            .index()
            .unwrap()
            .has_conflicts(),
        "index conflict entries must survive the refusal"
    );
}

#[test]
fn save_revision_refuses_with_lingering_merge_head_over_clean_index() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_lingering_merge_head(&root);
    fs::write(root.join("manuscript/one.md"), "post-damage typing\n").unwrap();
    let head_before = head_id(&root);

    assert_read_only_mentions(
        with_permit!(&root, |permit| git::save_revision(
            &root,
            "Auto: must refuse",
            permit
        )),
        "merge is in progress",
    );
    assert_eq!(head_id(&root), head_before);
    assert!(merge_head_id(&root).is_some());
}

#[test]
fn restore_refuses_mid_merge_with_zero_worktree_mutation() {
    // Round 9: a clean tree + lingering MERGE_HEAD passes the status-only
    // dirty check, and a refusal only inside the internal save_revision
    // would land AFTER the document was already replaced. The preflight
    // must fire before any disk write.
    // DRILL: move reject_merge_in_progress after the content write and the
    // snapshot assertions below fail.
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    let baseline = git::list_revisions(&root)
        .unwrap()
        .last()
        .expect("initial revision")
        .id
        .clone();
    make_lingering_merge_head(&root);
    let before = tree_snapshot(&root);

    assert_read_only_mentions(
        with_permit!(&root, |permit| git::restore_revision(
            &root, &baseline, permit
        )),
        "merge is in progress",
    );
    assert_eq!(
        before,
        tree_snapshot(&root),
        "restore_revision must refuse with zero worktree mutation"
    );

    assert_read_only_mentions(
        with_permit!(&root, |permit| git::restore_document(
            &root,
            "manuscript/one.md",
            &baseline,
            permit
        )),
        "merge is in progress",
    );
    assert_eq!(
        before,
        tree_snapshot(&root),
        "restore_document must refuse with zero worktree mutation"
    );
}

#[test]
fn manual_backup_mid_merge_skips_commit_but_still_pushes() {
    // Round 9: the push is benign (branch ref only, MERGE_HEAD is
    // local-only); refusing it would reduce backup protection during
    // exactly the window a writer wants an offsite copy.
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let head_before = head_id(&root);
    let backup_dir = tmp.path().join("backups");

    let revision = with_permit!(&root, |permit| git::backup_current_work(
        &root,
        &backup_dir,
        "Manual backup",
        permit
    ))
    .expect("backup must succeed mid-merge");

    assert!(revision.is_none(), "the commit half must be skipped");
    assert_eq!(head_id(&root), head_before);
    assert!(merge_head_id(&root).is_some());
    let bare = git2::Repository::open_bare(backup_dir.join("Novel.git")).unwrap();
    assert_eq!(
        bare.head().unwrap().target().unwrap(),
        head_before,
        "the push half must still deliver the branch ref"
    );
}

// ── complete_merge (round 8–9) ───────────────────────────────────────────

#[test]
fn complete_merge_commits_two_parents_clears_state_and_bumps_epoch() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let pre_head = head_id(&root);
    let pre_merge_head = merge_head_id(&root).unwrap();

    // The writer resolves the markers in the editor…
    fs::write(root.join("manuscript/one.md"), "# One\n\nResolved.\n").unwrap();
    // …and a session token from before completion must go stale after it.
    let session_token = tk(&root);

    let recovery = acquire_recovery_permit(&root).expect("recovery authority mid-merge");
    let revision =
        git::complete_merge(&root, "Merged incoming changes", &recovery).expect("complete");

    assert_eq!(head_parent_ids(&root), vec![pre_head, pre_merge_head]);
    assert!(merge_head_id(&root).is_none(), "merge state cleared");
    assert!(!git::merge_state(&root).unwrap().in_progress);
    assert_eq!(revision.message, "Merged incoming changes");

    let repo = git2::Repository::open(&root).unwrap();
    let tree = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .tree()
        .unwrap();
    let blob = repo
        .find_blob(tree.get_path(Path::new("manuscript/one.md")).unwrap().id())
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(blob.content()),
        "# One\n\nResolved.\n",
        "the merge commit must contain the resolution, not markers"
    );

    assert!(
        session_token.is_stale(),
        "completion mints a commit: outstanding authority must be refused"
    );
    assert!(
        !tk(&root).is_stale(),
        "a fresh probe re-authorizes normally after completion"
    );
}

#[test]
fn recovery_authority_is_only_issued_mid_merge() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    assert_read_only_mentions(
        acquire_recovery_permit(&root).map(|_| ()),
        "no merge is in progress",
    );
}

#[test]
fn recovery_authority_fails_closed_when_the_merge_ends_before_use() {
    // Rounds 12–13: between confirmation and use another process may
    // complete or abort the merge; the tree can then be clean and
    // Full-fidelity, so every ordinary check would pass while the
    // recovery action discards freshly committed state.
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let recovery = acquire_recovery_permit(&root).unwrap();

    // Another process aborts the merge…
    {
        let repo = git2::Repository::open(&root).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let mut co = git2::build::CheckoutBuilder::new();
        co.force();
        repo.reset(head.as_object(), git2::ResetType::Hard, Some(&mut co))
            .unwrap();
        repo.cleanup_state().unwrap();
    }
    let before = tree_snapshot(&root);

    // …and every held recovery action must refuse rather than act.
    assert_read_only_mentions(
        git::complete_merge(&root, "stale", &recovery).map(|_| ()),
        "no longer in progress",
    );
    assert_read_only_mentions(
        git::force_resolve_merge(&root, &recovery, None),
        "no longer in progress",
    );
    assert_read_only_mentions(
        git::sync_abort_pull(&root, &recovery),
        "no longer in progress",
    );
    assert_eq!(before, tree_snapshot(&root), "nothing may be reset");
}

#[test]
fn recovery_authority_is_root_bound() {
    let tmp = TempDir::new().unwrap();
    let root_a = init_project(&tmp);
    let root_b = tmp.path().join("Other.chikn");
    fs::create_dir_all(root_b.join("manuscript")).unwrap();
    git::init_repo(&root_b).unwrap();
    write_manifest(&root_b, "Other");
    fs::write(root_b.join("manuscript/one.md"), BASE_DOC).unwrap();
    fs::write(
        root_b.join("manuscript/one.meta"),
        "id: doc-one\ncreated: '2025-01-01T00:00:00Z'\nmodified: '2025-01-01T00:00:00Z'\n",
    )
    .unwrap();
    with_permit!(&root_b, |permit| git::save_revision(
        &root_b, "Initial", permit
    ))
    .unwrap();
    make_doc_conflict(&root_a);
    make_doc_conflict(&root_b);

    let recovery_a = acquire_recovery_permit(&root_a).unwrap();
    let before_b = tree_snapshot(&root_b);
    assert_read_only_mentions(git::sync_abort_pull(&root_b, &recovery_a), "granted for");
    assert_read_only_mentions(
        git::complete_merge(&root_b, "hijack", &recovery_a).map(|_| ()),
        "granted for",
    );
    assert_read_only_mentions(
        git::force_resolve_merge(&root_b, &recovery_a, None),
        "granted for",
    );
    assert_eq!(before_b, tree_snapshot(&root_b));
}

// ── Abort / force-resolve under recovery authority ──────────────────────

#[test]
fn abort_via_recovery_restores_local_and_bumps_epoch() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let session_token = tk(&root);

    let recovery = acquire_recovery_permit(&root).unwrap();
    git::sync_abort_pull(&root, &recovery).expect("abort");

    assert_eq!(
        fs::read_to_string(root.join("manuscript/one.md")).unwrap(),
        MASTER_DOC,
        "abort keeps the writer's own (pre-merge) version"
    );
    assert!(!git::merge_state(&root).unwrap().in_progress);
    assert!(
        session_token.is_stale(),
        "abort replaces the tree: outstanding authority must be refused"
    );
}

#[test]
fn force_resolve_takes_the_draft_tip_for_draft_conflicts() {
    // Round 12 (source-aware force): MERGE_HEAD is the draft being merged
    // after a draft conflict — there may be no sync remote at all, so the
    // old remote-only force target was wrong for this origin.
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let session_token = tk(&root);

    let recovery = acquire_recovery_permit(&root).unwrap();
    git::force_resolve_merge(&root, &recovery, None).expect("force resolve");

    assert_eq!(
        fs::read_to_string(root.join("manuscript/one.md")).unwrap(),
        DRAFT_DOC,
        "force takes the incoming (MERGE_HEAD) version wholesale"
    );
    assert!(!git::merge_state(&root).unwrap().in_progress);
    assert!(!git::has_changes(&root).unwrap());
    assert!(session_token.is_stale());
}

#[test]
fn force_resolve_fails_closed_when_confirmation_is_stale() {
    // Finding s4-1: the writer confirms the discard against the merge the
    // dialog SHOWED them. If the live merge no longer matches that
    // attestation — swapped underneath an open dialog — the force must
    // refuse rather than resolve a merge the writer never saw. Unparsable
    // attestations fail closed the same way.
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let real = git::merge_state(&root)
        .unwrap()
        .attestation
        .expect("mid-merge state carries an attestation");
    // Well-formed but naming a different merge (wrong OID, wrong print).
    let stale = format!("{}:{:016x}", head_id(&root), 0u64);
    let before = tree_snapshot(&root);

    let recovery = acquire_recovery_permit(&root).unwrap();
    assert_read_only_mentions(
        git::force_resolve_merge(&root, &recovery, Some(&stale)),
        "changed since you confirmed",
    );
    assert_read_only_mentions(
        git::force_resolve_merge(&root, &recovery, Some("not-an-attestation")),
        "could not be identified",
    );
    assert_eq!(before, tree_snapshot(&root), "nothing may be reset");
    assert!(merge_head_id(&root).is_some());

    // The matching attestation — what the dialog actually showed — passes.
    git::force_resolve_merge(&root, &recovery, Some(&real)).expect("force resolve");
    assert_eq!(
        fs::read_to_string(root.join("manuscript/one.md")).unwrap(),
        DRAFT_DOC
    );
}

#[test]
fn force_resolve_distinguishes_merges_of_the_same_incoming_commit() {
    // Finding s4-1, reopened round: aborting merge A and re-merging the
    // SAME draft against a different local state yields the same
    // MERGE_HEAD — an OID-only confirmation binding passes while the
    // writer confirmed a different merge. The attestation's fingerprint
    // half must catch it.
    // DRILL: compare only the OID half and this fails (the force
    // proceeds against the unconfirmed merge).
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let confirmed = git::merge_state(&root).unwrap().attestation.unwrap();
    let merge_head_a = merge_head_id(&root).unwrap();

    // Another process aborts, changes the LOCAL side, and re-merges the
    // same draft: same incoming commit, different merge.
    {
        let repo = git2::Repository::open(&root).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let mut co = git2::build::CheckoutBuilder::new();
        co.force();
        repo.reset(head.as_object(), git2::ResetType::Hard, Some(&mut co))
            .unwrap();
        repo.cleanup_state().unwrap();
    }
    fs::write(
        root.join("manuscript/one.md"),
        "# One\n\nMaster rewritten again.\n",
    )
    .unwrap();
    with_permit!(&root, |permit| git::save_revision(
        &root,
        "Master again",
        permit
    ))
    .unwrap();
    match with_permit!(&root, |permit| git::merge_draft(&root, "alt", permit)).unwrap() {
        git::MergeResult::Conflicts { .. } => {}
        other => panic!("second merge must conflict, got {other:?}"),
    }
    assert_eq!(
        merge_head_id(&root).unwrap(),
        merge_head_a,
        "fixture invariant: the incoming commit is identical across both merges"
    );

    let before = tree_snapshot(&root);
    let recovery = acquire_recovery_permit(&root).unwrap();
    assert_read_only_mentions(
        git::force_resolve_merge(&root, &recovery, Some(&confirmed)),
        "changed since you confirmed",
    );
    assert_eq!(before, tree_snapshot(&root), "nothing may be reset");
    assert!(merge_head_id(&root).is_some());
}

#[test]
fn force_resolve_fails_closed_on_staged_content_drift() {
    // Finding s4-2 (plan round 13: "index/worktree fingerprint"): restaging
    // different content externally while restoring the worktree bytes
    // leaves status bits AND worktree content unchanged — only the index
    // entry moves. The fingerprint must still drift, or the reset discards
    // newly staged work.
    // DRILL: drop the index entries from merge_fingerprint and this fails
    // (the force proceeds).
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);

    // A file staged once (INDEX_NEW) and then edited (WT_MODIFIED).
    let notes = root.join("manuscript/notes.md");
    fs::write(&notes, "staged-A\n").unwrap();
    {
        let repo = git2::Repository::open(&root).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("manuscript/notes.md")).unwrap();
        index.write().unwrap();
    }
    fs::write(&notes, "worktree-B\n").unwrap();

    let recovery = acquire_recovery_permit(&root).unwrap();

    // External restage: index entry becomes C while worktree bytes and
    // status bits end up exactly as fingerprinted.
    fs::write(&notes, "staged-C\n").unwrap();
    {
        let repo = git2::Repository::open(&root).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("manuscript/notes.md")).unwrap();
        index.write().unwrap();
    }
    fs::write(&notes, "worktree-B\n").unwrap();

    assert_read_only_mentions(
        git::force_resolve_merge(&root, &recovery, None),
        "changed since",
    );
    assert!(
        merge_head_id(&root).is_some(),
        "the merge stays in progress"
    );
    assert_eq!(
        fs::read_to_string(&notes).unwrap(),
        "worktree-B\n",
        "nothing may be reset on a failed re-attestation"
    );
}

#[test]
fn force_resolve_fails_closed_on_external_edits_after_confirmation() {
    // Round 13: an existence-only re-attestation passes while the
    // specific worktree changed — external partial resolution work done
    // between confirmation and the hard reset would be discarded.
    // DRILL: weaken the fingerprint to status-bits-only and this fails
    // (a content edit to an already-conflicted file keeps the same bits).
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let recovery = acquire_recovery_permit(&root).unwrap();

    let external = "# One\n\nExternally resolved while the dialog sat open.\n";
    fs::write(root.join("manuscript/one.md"), external).unwrap();

    assert_read_only_mentions(
        git::force_resolve_merge(&root, &recovery, None),
        "changed since",
    );
    assert_eq!(
        fs::read_to_string(root.join("manuscript/one.md")).unwrap(),
        external,
        "nothing may be reset on a failed re-attestation"
    );
    assert!(
        merge_head_id(&root).is_some(),
        "the merge stays in progress"
    );
}

#[test]
fn force_resolve_fails_closed_when_a_different_merge_started() {
    // Round 13: abort-then-different-merge — MERGE_HEAD exists both times
    // but points at a different commit; acting would resolve a merge the
    // writer never confirmed.
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_doc_conflict(&root);
    let recovery = acquire_recovery_permit(&root).unwrap();

    // Another process aborts, then starts a DIFFERENT merge (second
    // draft with its own conflicting edit).
    {
        let repo = git2::Repository::open(&root).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let mut co = git2::build::CheckoutBuilder::new();
        co.force();
        repo.reset(head.as_object(), git2::ResetType::Hard, Some(&mut co))
            .unwrap();
        repo.cleanup_state().unwrap();
    }
    let original = current_branch(&root);
    with_permit!(&root, |permit| git::create_draft(&root, "alt2", permit)).unwrap();
    fs::write(root.join("manuscript/one.md"), "# One\n\nSecond draft.\n").unwrap();
    with_permit!(&root, |permit| git::save_revision(
        &root,
        "Second draft edit",
        permit
    ))
    .unwrap();
    with_permit!(&root, |permit| git::switch_draft(&root, &original, permit)).unwrap();
    fs::write(root.join("manuscript/one.md"), "# One\n\nMaster again.\n").unwrap();
    with_permit!(&root, |permit| git::save_revision(
        &root,
        "Master again",
        permit
    ))
    .unwrap();
    match with_permit!(&root, |permit| git::merge_draft(&root, "alt2", permit)).unwrap() {
        git::MergeResult::Conflicts { .. } => {}
        other => panic!("second merge must conflict, got {other:?}"),
    }

    let before = tree_snapshot(&root);
    assert_read_only_mentions(
        git::force_resolve_merge(&root, &recovery, None),
        "changed since",
    );
    assert_eq!(before, tree_snapshot(&root));
    assert!(merge_head_id(&root).is_some());
}

// ── The live-bug regressions: format-file conflicts (rounds 9–12) ───────

#[test]
fn conflicted_project_yaml_can_still_be_aborted_after_restart() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_yaml_conflict(&root);

    // The restart boundary: the probe errors, so NO ordinary authority is
    // obtainable — this is exactly the stranding that was live.
    assert!(probe_project_fidelity(&root).is_err());
    assert!(acquire_write_token(&root).is_err());

    // The display path still opens (HEAD-metadata fallback)…
    let project = read_project_recovery(&root).expect("recovery open");
    assert_eq!(project.name, "Master Name", "metadata falls back to HEAD");

    // …and the recovery authority reaches the abort.
    let recovery = acquire_recovery_permit(&root).expect("recovery authority");
    git::sync_abort_pull(&root, &recovery).expect("abort");

    assert!(!git::merge_state(&root).unwrap().in_progress);
    assert_eq!(
        probe_project_fidelity(&root).unwrap(),
        Fidelity::Full,
        "aborting restores a fully healthy project"
    );
    assert!(!tk(&root).is_stale());
}

#[test]
fn conflicted_project_yaml_can_still_be_force_resolved() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_yaml_conflict(&root);
    assert!(acquire_write_token(&root).is_err());

    let recovery = acquire_recovery_permit(&root).unwrap();
    git::force_resolve_merge(&root, &recovery, None).expect("force resolve");

    let yaml = fs::read_to_string(root.join("project.yaml")).unwrap();
    assert!(
        yaml.contains("Draft Name"),
        "force takes the incoming (draft) version: {yaml}"
    );
    assert_eq!(probe_project_fidelity(&root).unwrap(), Fidelity::Full);
}

#[test]
fn conflicted_project_yaml_can_be_completed_after_manual_resolution() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_yaml_conflict(&root);
    let pre_head = head_id(&root);
    let pre_merge_head = merge_head_id(&root).unwrap();

    // The writer resolves project.yaml in an external editor…
    write_manifest(&root, "Resolved Name");

    // …and Complete is reachable even though the probe still cannot issue
    // an ordinary permit for a project whose merge touched format files.
    let recovery = acquire_recovery_permit(&root).unwrap();
    git::complete_merge(&root, "Merged name change", &recovery).expect("complete");

    assert_eq!(head_parent_ids(&root), vec![pre_head, pre_merge_head]);
    assert!(!git::merge_state(&root).unwrap().in_progress);
    assert_eq!(probe_project_fidelity(&root).unwrap(), Fidelity::Full);
    let project = read_project(&root).unwrap();
    assert_eq!(project.name, "Resolved Name");
}

#[test]
fn conflicted_meta_probes_degraded_and_recovery_still_works() {
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    make_meta_conflict(&root);

    match probe_project_fidelity(&root).expect("probe must classify, not error") {
        Fidelity::Degraded { .. } => {}
        Fidelity::Full => panic!("a conflicted sidecar must probe Degraded"),
    }
    assert!(acquire_write_token(&root).is_err());

    let project = read_project_recovery(&root).expect("recovery open");
    assert!(project.documents.contains_key("doc-one"));

    let recovery = acquire_recovery_permit(&root).unwrap();
    git::sync_abort_pull(&root, &recovery).expect("abort");
    assert_eq!(probe_project_fidelity(&root).unwrap(), Fidelity::Full);
}

// ── Recovery read: HEAD/worktree skew (rounds 10–11) ────────────────────

#[test]
fn recovery_read_loads_hierarchy_skew_as_unlinked_instead_of_failing() {
    // A remote delete/recreate at the same path gives the worktree sidecar
    // a different id than the (HEAD) hierarchy expects. The ordinary open
    // hard-errors — correct for a consistent tree — but mid-merge the tree
    // is definitionally inconsistent and the display-only recovery open
    // must not fail the whole project over it.
    let tmp = TempDir::new().unwrap();
    let root = init_project(&tmp);
    fs::write(
        root.join("manuscript/one.meta"),
        "id: doc-recreated\ncreated: '2025-06-01T00:00:00Z'\nmodified: '2025-06-01T00:00:00Z'\n",
    )
    .unwrap();

    let ordinary = read_project(&root);
    assert!(
        matches!(&ordinary, Err(ChiknError::InvalidFormat(m)) if m.contains("loaded as document")),
        "the ordinary open must keep strict matching: {ordinary:?}"
    );

    let project = read_project_recovery(&root).expect("recovery open tolerates skew");
    assert!(
        project.documents.contains_key("doc-recreated"),
        "the skewed document loads under its own id"
    );
    assert!(
        !project.documents.contains_key("doc-one"),
        "the hierarchy entry stays unlinked rather than faking a match"
    );
}
