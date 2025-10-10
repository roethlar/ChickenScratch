Here’s a straight, unvarnished review of the **Chicken Scratch** project as represented by your three documents: **PROJECT_SPECIFICATION.md**, **TECHNICAL_SUMMARY.md**, and **OPEN_QUESTIONS.md**.
I’ll break this into (1) macro-level evaluation, (2) technical architecture critique, (3) process and execution realism, (4) product-market coherence, and (5) direct answers to key open questions where possible.

---

## 1. Macro-Level Evaluation

### Strengths

* **Exceptionally well-structured documentation.** The spec, summary, and open questions form a coherent technical narrative—few projects at this stage are this clearly defined.
* **Unusually strong backend foundation.** The Rust core is clean, test-heavy, and security-conscious, which gives the project real engineering legitimacy.
* **Clear product vision.** “Scrivener parity + Git + AI” is a compelling, differentiable hook.
* **Technical self-awareness.** You identify trade-offs, dependencies, and unknowns rather than pretending they’re solved.

### Weaknesses

* **Scope bloat.** The spec describes three substantial products at once:

  1. a Scrivener-compatible format converter,
  2. a full cross-platform writing suite, and
  3. an AI assistant framework.
     Each could justify a standalone year-long roadmap. Combined, this is over-ambitious for a single-founder AI-driven workflow.
* **Frontend uncertainty.** You’ve replaced one UI stack (Tauri + React) with an undecided native-UI approach but haven’t reconciled that with the “AI-friendly” modular design assumption built for Tauri/React. That’s an architectural fracture point.
* **Dependence on Pandoc.** Excellent engineering shortcut, but a real UX liability if the target audience includes non-technical writers who will balk at CLI dependencies.
* **Testing asymmetry.** Backend testing is first-rate; frontend and E2E are vapor. The “100% AI-coded” constraint will make that gap widen fast.

### Verdict

A highly competent architecture document for a **Phase-1 proof-of-concept**—but it reads as a **Phase-4 product roadmap**. You need to narrow the aperture drastically if you want something to ship before it fossilizes.

---

## 2. Technical Architecture Critique

### Core / Backend

* **Rust choice:** sound. Memory safety and performance are critical for file I/O heavy apps.
* **File format:** `.chikn` is well-conceived—human-readable, Git-friendly, lossless. Keep it.
* **Scrivener compatibility:** feasible but non-trivial. RTF round-tripping via Pandoc will always have edge-case losses (especially custom styles and inline images). Expect “almost-lossless,” not perfect fidelity.
* **Git integration:** calling `git2` is straightforward, but a writer-friendly UX around it is non-trivial; merge resolution for text + RTF is a multi-month problem.
* **AI layer:** easy to integrate technically (API wrappers), but context management (what the model sees, token costs, privacy) is product-level complexity.

### Interop & UI Binding (from OPEN_QUESTIONS.md)

* **FFI vs UniFFI:** choose **UniFFI**. The project’s multi-UI vision needs safe, auto-generated bindings. Manual FFI will drown you in boilerplate.
* **Serialization:** structured data via UniFFI, not JSON, once stable. Start with JSON only if you need to demo quickly.
* **Tauri:** defer but don’t delete. React web UI remains your best bet for future SaaS pivot.
* **SwiftUI-first:** correct decision—validate UX once, clone patterns later.
* **Linking:** static for simplicity (desktop distribution), dynamic only if you plan plugin systems.
* **Error handling and memory:** rely on UniFFI; don’t hand-roll unsafe pointer passing.
* **Async:** keep Rust synchronous; dispatch to threads in Swift/C#. Async interop is fragile.
* **Mono-repo:** fine for your current solo+AI workflow.
* **Testing balance:** your proposed split (heavy core tests, light bindings, UI automation later) is appropriate.

### Gaps / Missing Abstractions

* **Configuration management.** No mention of centralized settings schema across languages—decide now before you end up with platform-specific drift.
* **Data migration/versioning.** `.chikn` schema evolution strategy missing; YAML alone won’t suffice long-term.
* **Plugin extensibility.** Post-1.0 maybe, but you should at least reserve a namespaced folder for later modules.

---

## 3. Process and Execution Realism

### Timeline

Your seven-phase roadmap is **fantasy-level optimistic** unless you’re offloading 95% of implementation to AIs and you act as full-time reviewer. Even then, quality-assurance and integration will eat time.

Realistic compression:

| Phase         | Original | Realistic single-developer timeline |
| ------------- | -------- | ----------------------------------- |
| 1             | 2 months | 2–3 months                          |
| 2             | 2 months | 3–4 months                          |
| 3 – 4 – 5 – 6 | 8 months | 12–16 months total                  |
| Launch        | 1 month  | +2 months stabilization             |

Expect **18 months minimum** for credible v1.0.

### “100% AI-coded”

That’s a serious bottleneck. AI can write isolated modules well; it struggles with multi-crate integration, CI/CD maintenance, and subtle UX consistency. Treat “AI-coded” as “AI-drafted, human-curated.” Otherwise you’ll spend more time debugging generated glue than writing features.

### Learning vs Review Mode

If you want production software, operate in **review-only** mode. If you want to learn Rust/Swift, halve your deliverables. You can’t do both simultaneously at this scope.

---

## 4. Product–Market Coherence

### Market Fit

There’s a clear niche: writers fleeing macOS/Scrivener who want Linux parity. That’s small but real.
However, layering Git and AI on top shifts the product from “replacement” to “experimental studio.” That confuses the pitch. Pick a flagship story:

* **Scrivener replacement first.**
* Git and AI are secondary differentiators introduced post-launch.

### UX Philosophy

Your “meat vs gravy” analogy is sound: feature parity is the meat; native look-and-feel is gravy. Enforce functional equivalence, not visual uniformity. Each platform’s UI should feel native.

### Business Viability

The spec straddles open-source ethos and commercial polish. Choose one early:

* OSS path → prioritize portability, minimize proprietary integrations, community adoption.
* Paid desktop app → emphasize SwiftUI polish, installers, App Store compliance.
  The architecture can support either, but the go-to-market narrative must be singular.

---

## 5. Answers to Key Open Questions

| Question                      | Direct Answer                                                 |
| ----------------------------- | ------------------------------------------------------------- |
| **Q1 FFI vs UniFFI**          | **UniFFI.** Time-saving, safer, future-proof.                 |
| **Q2 Serialization**          | Structured types via UniFFI; JSON only for early prototyping. |
| **Q3 Tauri**                  | Keep dormant; defer. Extract core library cleanly first.      |
| **Q4 SwiftUI-first**          | Yes—one platform reference UI.                                |
| **Q5 Swift package vs Xcode** | Swift Package Manager for reuse and CI simplicity.            |
| **Q6 Dynamic vs static**      | Static linking for initial desktop releases.                  |
| **Q7 Memory mgmt**            | Let UniFFI handle ref-counted ownership.                      |
| **Q8 Error propagation**      | UniFFI auto-conversion; use rich Rust `Error` enums.          |
| **Q9 Async/await**            | Expose sync API; dispatch async in host.                      |
| **Q10 Repo layout**           | Mono-repo until you onboard other devs.                       |
| **Q11 Versioning**            | Synchronized for 1.0, decouple later.                         |
| **Q12 Testing balance**       | Keep heavy Rust tests; minimal binding smoke tests.           |
| **Q13 CI/CD scope**           | Linux-only CI initially; manual macOS/Windows.                |
| **Q14 UI style**              | Feature-parity identical, visuals platform-native.            |
| **Q15 Focus modes**           | Behaviorally consistent, visually adaptive per platform.      |
| **Q16 Hands-on level**        | If shipping soon → review-only.                               |
| **Q17 “100% AI-coded”**       | Interpret as “AI-generated, human-approved.”                  |
| **Q18 Revenue model**         | Optimize for paid desktop first; cloud later if traction.     |

---

## 6. Final Appraisal

* **Engineering quality:** 8.5 / 10
* **Architectural clarity:** 9 / 10
* **Scope realism:** 4 / 10
* **Market focus:** 7 / 10
* **Execution risk:** High but manageable with ruthless prioritization.

**If you want this to ship:**

1. Freeze scope at *Scrivener-compatible writer with minimal AI assist.*
2. Keep Rust core, build SwiftUI MVP via UniFFI.
3. Defer Git UI and full AI assistant until 1.1+.
4. Treat Pandoc as a stopgap, not a permanent dependency.

That path gives you a defensible, demo-ready product within a year and preserves your long-term ambitions without collapsing under them.

"""
User:
if you were presented with this project at the conceptual phase, with only the goals and none of the code, what would your design be? greenfield.
"""

Good — greenfield design, straight to the point. I’ll assume only the goals from your docs: **Scrivener parity + a git-friendly, human-readable project format (.chikn) + cross-platform native apps + optional web + AI assistant support + high data integrity**. Below is a complete, opinionated design I would implement if handed this at concept stage.

# High-level decisions (non-negotiable)

* **Core runtime:** Rust (single source of truth for file format, conversions, git, and business logic). Rationale: safety, performance, single language for desktop cores, and easy to compile to native libs.
* **Bindings:** **UniFFI** to expose a typed API to platform frontends (SwiftUI, WinUI/.NET, GTK). Rationale: auto-generated bindings, fewer interop bugs, faster multi-platform engineering.
* **UI approach:** **SwiftUI-first native** for macOS, **WinUI (.NET/WinRT)** for Windows, **GTK4 (Rust or Swift via bindings)** for Linux. Keep a Tauri/React web UI as optional/future — keep codepath alive but frozen until native MVP proves product-market fit.
* **Project format:** `.chikn` as a directory of text files (Markdown + YAML metadata + assets). Enforce strict schema and versioned migrations.
* **RTF/Word conversions:** Use **Pandoc** as first implementation for conversions, but design an abstraction layer so Pandoc can be swapped for a native converter later.
* **AI layer:** Pluggable provider adapter pattern (OpenAI, Anthropic, Ollama, LocalAI). All AI calls go through the core with explicit user-controlled context windows and privacy toggles.

# Core architecture (single source diagram)

```
┌──────────────────────────┐
│   Native Frontends       │  (SwiftUI, WinUI, GTK)
└──────────┬───────────────┘
           │ UniFFI bindings (typed)
┌──────────▼───────────┐
│  Platform adapters   │  (UI thread helpers, async -> sync)
└──────────┬───────────┘
           │
┌──────────▼──────────┐
│  chicken-core (Rust)│  <-- authoritative logic
│  - .chikn schema    │
│  - I/O, atomic writes│
│  - Scrivener import/export abstraction
│  - Git integration  │
│  - AI provider adapters (thin)
│  - Migration/versioning
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│   Native libs &     │  (Pandoc wrapper, sqlite for snapshots, logging)
└─────────────────────┘
```

# API & boundaries

* The core exposes a **small, well-typed surface** via UniFFI. Keep it to ~20 high-value calls.

  * `open_project(path: String) -> Project`
  * `create_project(path, metadata) -> Project`
  * `list_documents(project_id) -> Vec<DocumentRef>`
  * `read_document(project_id, doc_id) -> Document`
  * `write_document(project_id, Document) -> Result`
  * `import_scrivener(path) -> ImportReport`
  * `export_scrivener(project_id, path) -> ExportReport`
  * `git_init(project_id)`, `git_status(project_id)`, `git_commit(project_id, msg)`
  * `ai_request(project_context, prompt, provider_config) -> AiResponse`
* Keep the API **synchronous** from the UI’s perspective. The frontends will call on background threads; avoid crossing async runtimes.

# Data format & versioning

* `.chikn` is a directory:

  * `project.yaml` (versioned schema, metadata)
  * `manuscript/` (files `NN-title.md` + `NN-title.meta.yaml`)
  * `research/` (assets in `assets/`, with metadata pointing to relative paths)
  * `templates/`, `settings/`
  * `.git/` optional (support both internal snapshots and external git)
* **Schema versioning:** `project.yaml` carries `format_version` and a `migrations/` manifest. Implement deterministic, idempotent migrations in Rust.
* **File naming:** deterministic slugging with collision resolution. Keep document UUIDs recorded in `.meta` files to preserve Scrivener mapping.
* **Snapshots:** lightweight snapshot store implemented on top of git or an internal sqlite if user opts out of git.

# Conversion strategy

* Implement an **adapter layer** `ConversionAdapter` with a Pandoc-backed implementation initially. Keep an abstraction so a different converter can be plugged later.
* For import/export:

  * Parse Scrivener `.scrivx` XML -> map binder structure -> import RTF via Pandoc -> produce `.chikn`.
  * On export, produce RTF via Pandoc -> build `.scrivx` and `Files/Data/...` layout.
* **Expectation management:** perfect fidelity is impossible. Design tests and heuristics for “loss risk” reporting and provide warnings in UI for fields that can’t round-trip exactly.

# Git & version control

* Provide optional git init per project. Default: no git unless user enables.
* Offer two UX modes: “Writer-friendly” (Revisions, Auto-commit toggles) and “Power mode” (real commits, branch management).
* Conflicts: handle per-file merges (Markdown) with a basic three-way merge UI; for RTF/complex binary formats, surface a guided manual resolution workflow.

# AI integration design

* **Adapter pattern**: `AiProvider { prepare_context, call_model, post_process }`
* **Context management**: Users select scope (current doc, chapter, character sheets, full project). Core returns a bounded context package and token estimate. Enforce explicit user consent before sending anything to cloud providers.
* **Local-first option:** Prioritize Ollama/LocalAI for privacy-conscious users; ensure API parity in adapters.
* **Safety & cost controls:** per-project quotas, cost estimates, and “dry-run” mode.

# Error handling & reliability

* Core uses rich `Result` enums with contextual error types (IOError, SchemaError, ConvertError, ProviderError, GitError).
* Bindings convert Rust errors into native exceptions / Result types. Use UniFFI default conversion.
* Atomic writes: always write to temp file and `rename` (POSIX atomic). Fallback to transactional write on Windows.
* Robust path validation and sandboxing; no directory traversal.

# Testing strategy

* **Rust core:** heavy unit tests, property tests, fuzz tests for parsers, integration tests with real Scrivener files. Block-level coverage target high.
* **Bindings:** smoke tests for each target language (open->read->write->close roundtrip).
* **UI:** Playwright for web UI if used; platform UI automation (XCUITest for mac, WinAppDriver for Windows, dogtail/LDTP for Linux) for critical flows only.
* **Conversion tests:** golden files and round-trip tests (import -> export -> compare essential fields).
* CI runs Rust tests + Linux smoke binding tests on every PR. Platform builds gated to release artifacts.

# Security and privacy

* No default telemetry. Opt-in only, with visible UI consent. Keep analytics anonymized and aggregated.
* Encryption at rest: optional per-project encryption (password-derived key; encrypt whole project directory when enabled).
* Clear UI warnings when cloud AI providers are used.

# Developer ergonomics & DX

* Mono-repo with workspace crates: `core`, `conversions`, `git`, `ai`, `bindings/udl/`, `cli`.
* Strict linting, formatting, and pre-commit hooks.
* Provide `chikn-cli` for power users: create/open project, run import/export, run snapshots, run AI tasks — helps testers and CI.

# MVP (must-have to call it an MVP)

* Core: `.chikn` read/write, atomic save, metadata, slugging, project open/create.
* Scrivener import (read-only) of typical Scrivener 3 project and create `.chikn` (RTF->Markdown via Pandoc).
* SwiftUI native app: navigator + editor + inspector + open/save + basic preferences.
* UniFFI bindings for Swift (and a minimal C# binding smoke test).
* Tests: core unit tests + integration import test with one real .scriv sample.
* Documentation: format spec, API surface, developer README.

# Phased rollout (no durations — just phases)

1. Core + Schema + CLI + Unit tests
2. SwiftUI frontend connected via UniFFI (MVP)
3. Scrivener import/export refinement + round-trip tests
4. Git opt-in + writer-friendly UI
5. AI integration + provider adapters
6. Platform parity (WinUI + GTK)
7. Polish, accessibility, packaging, release channels

# Observed risks & mitigations

* **Conversion edge-cases (RTF styles)** → mitigation: surfacing “loss risk” and conservative conversion modes; build sample corpus.
* **Pandoc dependency friction** → mitigation: ship platform-specific installer helpers + bundle Pandoc on desktop installers where license allows; design for replaceability.
* **Multi-UI divergence** → mitigation: strict ADRs + shared design tokens & UX guidelines; implement feature flags and a single canonical UX flow documented in SwiftUI reference.
* **“100% AI-coded” misconception** → mitigation: use AI to scaffold components and tests, but require human review gates for integration and UI polish.
* **Legal risk re: Scrivener** → mitigation: clean-room implementation, no reuse of Scrivener code; clearly document interoperability goal.

# Operational and delivery needs (roles)

* **Core engineer (Rust)** — author of core library and bindings
* **Frontend engineer (SwiftUI)** — implement native mac app and iterate UX
* **Conversion engineer / QA** — test Scrivener corpus and improve conversion heuristics
* **DevOps / CI** — cross platform build matrix, packaging
* **Product / Writer liaison** — validate parity and usability with real writer beta testers

# Example UniFFI UDL (short)

```idl
namespace chickenscratch;

dictionary Project {
  string id;
  string path;
  string name;
  u32 format_version;
}

dictionary Document {
  string id;
  string title;
  string slug;
  string content; // Markdown
  map<string, string> metadata;
}

// Core API
Project open_project(string path);
Project create_project(string path, string name);
sequence<Document> list_documents(string project_id);
Document read_document(string project_id, string document_id);
void write_document(string project_id, Document doc);
string import_scrivener(string scriv_path); // returns new project path
```

This is intentionally small — keep surface area minimal and iterate.

# Deliverables I’d expect from the conceptual phase (to start implementing)

* Finalized `.chikn` schema v0.1 and migration design
* Canonical API surface (UDL) locked for v0.1
* Conversion adapter interface + initial Pandoc adapter
* SwiftUI wireframes (navigator+editor+inspector)
* Sample Scrivener corpus (10 diverse projects) for conversion tests
* CI matrix config for core tests + binding smoke tests

---

# Final note — tradeoffs summarized

* **Speed → Pandoc + UniFFI + SwiftUI-first.** This gets you to a credible demo sooner with less low-level plumbing.
* **Long-term fidelity → invest later in native conversion once you have user cases.**
* **Cross-platform parity → one reference UI (SwiftUI) that defines behavior; other frontends aim for behavioral parity, not pixel-perfect clones.**

