# Aether — Technical Debt

Known limitations, simplifications, and planned refactors. Technical debt is
never left undocumented: when a milestone takes a deliberate shortcut, it is
recorded here with enough context to pay it down later.

Each item notes its **impact**, the **trigger** that should prompt action, and a
**severity** (low / medium / high).

---

## Open items

### TD-0001 — Hand-rolled CLI argument parsing
- **Severity:** low
- **Context.** `aetherc` parses arguments by hand (see `crates/aetherc/src/cli.rs`)
  to keep the foundation dependency-free (ADR-0007). It supports only
  `--help`/`-h`, `--version`/`-V`, and a single input path.
- **Impact.** Fine today; will not scale to subcommands, grouped flags, or
  `--`-style option termination.
- **Trigger.** The first time we need subcommands (e.g. `aetherc build`,
  `aetherc emit-air`) or more than a handful of flags, migrate to `clap` (derive).
- **Notes.** `--` (end-of-options) is currently treated as an unknown option;
  fold this into the `clap` migration.

### TD-0002 — `aetherc` compilation pipeline is a stub
- **Severity:** low (by design)
- **Context.** The `compile` path reads the input file and reports that the
  pipeline is unimplemented (exit code 3).
- **Impact.** No compilation happens yet.
- **Trigger.** Resolved incrementally across Phase 1 (M1–M5); fully paid down when
  M5 wires the interpreter through the driver.

### TD-0003 — Single MIT license
- **Severity:** low
- **Context.** The project is MIT-licensed for simplicity (ADR-0007).
- **Impact.** The Rust-ecosystem norm is dual MIT/Apache-2.0 (the Apache half adds
  an explicit patent grant).
- **Trigger.** Before any significant external contribution or public release,
  decide whether to dual-license; if so, add `LICENSE-APACHE`, update
  `license = "MIT OR Apache-2.0"`, and note it in `CONTRIBUTING.md`.

### TD-0004 — CI runs a single toolchain on a single OS
- **Severity:** low
- **Context.** CI builds/tests with the pinned stable toolchain on
  `ubuntu-latest` only.
- **Impact.** No coverage of macOS/Windows or of the declared MSRV (1.89) drifting
  from the pinned toolchain.
- **Trigger.** When the platform stabilizes, expand to an OS matrix and add an
  explicit MSRV job.

### TD-0005 — CI lint gating relies on `RUSTFLAGS`-free clippy step
- **Severity:** low
- **Context.** Lints are gated by the `clippy -- -D warnings` step; build/test do
  not force `-D warnings` (so future dependency warnings do not fail CI).
- **Impact.** A rustc-only warning that clippy somehow does not surface would not
  fail the build/test steps.
- **Trigger.** Revisit if we observe warnings escaping the clippy gate; consider a
  scoped deny via `[lints]` levels instead of a global flag.

### TD-0006 — `Span` is 12 bytes; rustc-style packing deferred
- **Severity:** low
- **Context.** `Span { file: FileId, lo, hi }` uses three `u32`s (ADR-0008). rustc
  packs file+range into 4 bytes (inline for small spans, interned otherwise).
- **Impact.** More memory per token/AST/IR node than a packed scheme.
- **Trigger.** If span storage shows up in profiling once real programs compile,
  pack the representation. `Span`'s private fields make this a non-breaking change.

### TD-0007 — Diagnostic renderer: single-line, ungrouped
- **Severity:** low
- **Context.** `render` underlines only the first line of a multi-line span and
  renders one snippet block per label (labels on the same line are not grouped).
- **Impact.** Multi-line spans and multi-label-per-line diagnostics look less
  polished than rustc's.
- **Trigger.** When diagnostics commonly span multiple lines (e.g. block/type
  errors), add multi-line rendering and per-line label grouping.

### TD-0008 — Columns count characters, not display width
- **Severity:** low
- **Context.** A column is one Unicode scalar value; double-width (CJK) and
  zero-width glyphs, and tab display width, are not accounted for in the reported
  column number.
- **Impact.** Caret alignment can drift for such glyphs (tabs are handled in the
  caret pad, but not in reported columns).
- **Trigger.** Add Unicode-width-aware column computation if/when it matters for
  users; consider the `unicode-width` crate.

### TD-0009 — No colored diagnostic output
- **Severity:** low
- **Context.** `render` emits plain text so output is stable to snapshot-test and
  safe to pipe.
- **Impact.** Less scannable in a terminal than colored output.
- **Trigger.** Add ANSI coloring gated on stderr being a TTY (and a `--color`
  flag), keeping a plain-text path for tests.

### TD-0010 — No `Session`/context type yet
- **Severity:** low
- **Context.** `ARCHITECTURE.md` envisions a `Session` owning per-compilation
  state (source map, diagnostics, interners, config). Today the driver holds a
  `SourceMap` and `DiagnosticHandler` directly.
- **Impact.** None yet; introducing it now would be premature abstraction.
- **Trigger.** Introduce `Session` when a third shared component (e.g. a string
  interner) and multiple phases need to thread the same state — likely M2.

---

## Resolved items

_None yet._
