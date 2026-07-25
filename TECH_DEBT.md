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

---

## Resolved items

_None yet._
