# Plan: CI green again under rustc 1.99 (Windows link, macOS kill race)

**Owner request (quote):**
> fix ci

(2026-10-24, in reply to the words-first report that the GitHub Validation run
`40071873531` was red for two pre-existing engine reasons and that "the
Windows link fix first, then dig into the macOS one" was the proposed action.)

**Phase check:** [x] Allowed by `CURRENT_PHASE.md` (hardening: CI gate is
load-bearing)  [x] Not paused

**Invariants touched:** I9 (verify before done — the CI gate itself). No
format, git-write or data-loss surface. I2 untouched (no `.chikn` I/O moves).

---

## [MODEL] Intent

The Validation workflow passes on all three runners with the current stable
toolchain, with no change to what the engine does for writers. Two
single-concern commits, one per failure.

## [MODEL] Approach

1. **Windows: link `advapi32` explicitly.** `libgit2-sys 0.17.0+1.8.1` calls
   registry, CryptoAPI and security-info functions from C without declaring
   the import library. It linked before only because Rust's std pulled
   `advapi32.lib` in; std 1.99 now binds that DLL through `raw-dylib`
   (`library/std/src/sys/pal/windows/c/windows_sys.rs`), so the import
   library left the linker line. A `crates/core/build.rs` emits
   `cargo:rustc-link-lib=advapi32` when the target OS is Windows. Native-lib
   flags on an rlib carry into every binary that links it, so the TUI,
   converter and web binaries are covered too. Alternative kept for later:
   upgrade `git2` 0.19 → 0.21 (API review needed; separate slice).
2. **macOS: treat `EPERM` from `kill(-pgid)` as "group already gone" once the
   leader is confirmed exited, allowing the exit-in-progress window.** XNU's
   `killpg1` returns `EPERM` (not `ESRCH`) when a process group has no live
   member it may signal, and it marks a process exited (`P_LIST_EXITED`)
   before the parent can reap it. The existing guard accepts `EPERM` only
   when `try_wait()` already reports an exit; the combined-output test loses
   that race when the helper finishes its 4 KiB and exits just as the cap
   trips. Fix: on `EPERM`, poll `try_wait()` for a short bounded window
   (500 ms, 2 ms steps) and accept the error only if the leader is seen to
   exit; a leader that stays alive still surfaces the error. Linux never
   returns `EPERM` here for a child we own, so the branch is inert there.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/core/build.rs` | new: Windows-only `rustc-link-lib=advapi32` |
| `crates/core/src/utils/process.rs` | `kill_tree` (unix): bounded wait for leader exit on `EPERM` |
| `.agents/state.md`, `docs/plans/README.md` | paperwork |

## [MODEL] Tests

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked -p chickenscratch-core --all-targets -- -D warnings`
- [ ] `cargo test --locked -p chickenscratch-core --lib utils::process`
- [ ] Declared suite minus the Tauri crate on the Linux box
- [ ] Real proof is the GitHub Validation run on `windows-2025-vs2026` and
      `macos-26-arm64` after the push (push needs its own go).

## [MODEL] Owner verification (plain English)

"After the push, the GitHub Actions page shows the Validation run green on
Linux, Windows and macOS."

## [YOU] Decisions needed

- Push go (per `.agents/push-policy.md`): CI is the only place these two
  fixes can be proven.

## Status

- 2026-10-24: both commits landed locally; awaiting push go.
