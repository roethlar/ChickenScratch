# plan-3: Review of PLAN_SCRIVENER_FIDELITY.md

**Type**: plan review (adapted from the code-finding flow as plan-1/plan-2
were; no branch, no guard proof — the artifact is a plan document on
`master`. The doc-review analog of the guard proof: the reviewer verifies
the plan's factual claims against the code and the sample projects.)
**Status**: In progress — round 1 reopened 2026-10-25; plan revised; round 2
pending
**Subject**: `docs/plans/PLAN_SCRIVENER_FIDELITY.md`
**Owner go**: "plan, review the plan with gpt-6-astra xhigh, then begin once
you have consensus. review code slices with the same codex model"
(2026-10-25)

## Round 1 dispatch

- **Reviewer**: codex-cli 0.164.0 (the MCP codex server failed to connect
  this session: CONNECTION_CLOSED), `codex exec --ephemeral -s read-only
  -C <repo> -m gpt-6-astra -c model_reasoning_effort=xhigh -o <file> -`
  with the prompt on stdin. Transcript header: `model: gpt-6-astra`,
  `provider: openai`, `reasoning effort: xhigh`, `sandbox: read-only`,
  session `01a174c6-5fed-71e1-b70f-4ff3c7c5a6ed`.
- **Tier**: frontier (owner-named pair gpt-6-astra @ xhigh; grade word not
  stated by the owner)
- **Reviewed SHA**: `604791d254500e067d9f6d2dfbf7d7b9f4fe0e52` (plan commit)
- **Base SHA**: `a9b76dd7aa6c992340f6d9ee43d1c37ef8edf4c3` (parent; plan
  absent there)
- **Dispatched**: 2026-10-25, finished 20:41 UTC; bound 2400 s; exit 0;
  136,376 tokens.
- **Capability proof**: read the plan file (matched the pinned head) and ran
  `git rev-parse HEAD` → `604791d…`; both succeeded. The reviewer also ran
  Pandoc read-only on all 21 sample RTF files, reproduced the Zoe failure
  and confirmed the four-paragraph result of the `\par` preparation. It
  could not run the converter or the suite (read-only sandbox forbids
  temporary files).
- **Verdict**: `reopened`, eight findings.

## Round 1 findings and dispositions

Each checked against the working tree before acceptance.

1. **Hex-escape rewrite corrupts Unicode fallbacks — HIGH — ADMITTED.**
   RTF `\uN` is followed by `\ucN` fallback items (default one) that a
   Unicode-aware reader must skip; rewriting a fallback `\'e9` to `\u233?`
   doubles the character (`cafééfin`). Plan: tokenizer-aware prepare step
   that keeps `\uN` and replaces its fallback sequence with `?`, tracked per
   group; exact-output tests added.
2. **Text-item children not converted — HIGH — ADMITTED.** Confirmed at
   `crates/cli/src/scrivener/converter/mod.rs` `"Text" =>` branch: no
   recursion into `item.children`; only `build_uuid_map` recurses. The
   plan's "already handled" was wrong. Plan: convert as folder-with-text,
   test text + folder descendants.
3. **`IncludeInCompile` parsed but never applied — HIGH — ADMITTED.**
   Confirmed: `Document { … ..Document::default() }` leaves
   `include_in_compile: true`. Plan: map it, test persistence and compile
   exclusion.
4. **Inline annotations are literal text, not RTF groups — HIGH —
   ADMITTED (plausible; no sample carries one).** Scrivener documents its
   inline annotation/footnote syntax as literal text with escaped braces
   so other RTF apps display it. Plan: handle the literal form after
   conversion and the raw-group form in the prepare step; hand-written
   fixtures for both; a Scrivener-generated document is the owner's
   verification item.
5. **Plain-text fallback destroys `scrivcmt://` anchors — HIGH — ADMITTED.**
   The fallback dropped `{\*\fldinst}`. Plan: the extractor turns `\field`
   groups into Markdown links; anchors are gathered before the lossy step;
   every sidecar body is reconciled, unanchored ones kept and warned.
6. **Default label is not the empty label — MEDIUM — ADMITTED.** Only the
   `-1` sentinel ("No Label"/"No Status") is none; `DefaultLabelID` is a
   real choice and resolves normally. Tests for both.
7. **Same-titled custom fields collide — MEDIUM — ADMITTED.** Colliding
   titles key as `custom.<title>.<id>`; test with two.
8. **I2 wording vs converter location — MEDIUM — ADMITTED as a
   documentation contradiction, flagged to the owner.** `docs/INVARIANTS.md`
   I2 and ADR-001 say Scrivener import lives in core; I2's own table,
   `docs/ARCHITECTURE.md`, the code and every accepted Scrivener fix
   (ar-1..ar-4) place it in `crates/cli`. The plan follows the code and
   records the ruling needed; it moves no code between crates.

**Plan revision**: committed for round 2 (see the plan's Status).
