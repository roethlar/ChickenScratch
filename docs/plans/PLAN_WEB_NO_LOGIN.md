# Plan: Web shell without a login

**Owner request (quote):**
> get rid of the access token. I didn't ask for that.

(2026-10-24, right after the installer slices landed.)

**Phase check:** [x] Allowed — follows the web-shell side quest; no phase
change  [x] Not paused

**Invariants touched:** none of I0–I10. The projects-root gate (I2-adjacent
confinement) stays; only the login goes.

---

## [MODEL] Intent

The web version opens straight to the projects list for anyone who reaches
its address. No token to copy, no gate screen, no token file. The owner's
private network / VPN is the only boundary, and the docs say so plainly.

## [MODEL] Approach

Remove, do not disable: `crates/web/src/auth.rs` and its middleware, the
`token` field on `AppState`, the `--token` flag / `CHIKN_WEB_TOKEN` /
`web-token` file and the tokenised "Open:" link in `main.rs`; the UI's token
storage, `Authorization` header, 401 handling and `WebTokenGate`; the
installer's token read-back. The per-tab `X-CS-Session` header stays (it
drives the single-writer lease, not security). Tests follow: the HTTP
contract tests drop the bearer header and the 401 test; the UI tests drop
the gate and the token-from-URL cases.

## [MODEL] Files

| File / area | Change |
|-------------|--------|
| `crates/web/src/{auth.rs,app.rs,state.rs,main.rs,lib.rs,config.rs}`, `Cargo.toml` | token removed; `subtle`/`uuid` deps dropped if unused |
| `crates/web/tests/http.rs` | no bearer header, no 401 test |
| `ui/src/commands/transport.ts`, `ui/src/components/welcome/{Welcome,WebTokenGate}.tsx`, tests, `editor.css` | token storage, gate and styles removed |
| `scripts/install-web.sh`, `scripts/uninstall-web.sh` | print the address, no token file |
| `docs/USER_GUIDE.md`, `README.md`, `RELEASE.md`, ADR-007, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `TODO.md`, `.agents/decisions.md`, `.agents/state.md` | say there is no login |

## [MODEL] Tests

- [x] `cargo clippy/test -p chickenscratch-web`
- [x] UI lint, build, tests
- [x] shellcheck on the scripts
- [x] Live smoke: start `chikn-web`, `GET /api/info` with no header → 200

## [MODEL] Owner verification (plain English)

"Open the server's address in a browser: the projects list appears with no
token step."

## [YOU] Decisions needed

- None. Recorded in `.agents/decisions.md`.

## Status

- 2026-10-24: done in one commit on the owner's instruction.
