# Machine-Specific Facts

Dated notes per machine. Prune stale entries during catchup hygiene sweeps.

## nagatha (macOS; previously recorded as nagatha.local)

- 2026-09-17: `~/Dev/scrivner` exists. It holds the owner's original
  Scrivener projects; validation must use copies. The chosen source and
  script are in `.agents/break-it-validation.md`.
- 2026-09-25: `~/chikn-validation/Corn.chikn` recreated from
  `samples/Corn.chikn` for the GUI rerun; disposable test data.
- 2026-09-17: `~/chikn-validation/` no longer exists. The historical
  post-step-9 test project recorded on 2026-08-29 is not available at its
  recorded location; recreate the sandbox before resuming validation.
- The earlier Claude Cowork driver limitation is superseded for planning
  purposes by the successful desktop-driven run recorded in
  `.agents/break-it-validation.md`; that run did not record the app's
  source revision.

## magneto (Linux dev box, Arch)

- 2026-10-24, rechecked 2026-10-25 (`pkg-config --exists`): `webkit2gtk-4.1`
  and `javascriptcoregtk-4.1` are not installed, so the Tauri crate
  (`cargo check -p chickenscratch`) fails in the `javascriptcore-rs-sys`
  build script. Installing the WebKitGTK dev package is an owner call
  (system change); CI's macOS runner is the real check
  (`.agents/state.md`, Known drift).
