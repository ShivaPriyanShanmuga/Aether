# Aether — Project Status

**Snapshot date:** 2026-07-24
**Current phase:** Phase 0 — Foundation
**Current milestone:** M0 — Project foundation → ✅ **complete**
**Next milestone:** M1 — Source & diagnostics infrastructure

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Completed milestones

- **M0 — Project foundation** ✅
  - Cargo workspace with pinned toolchain (`rust-toolchain.toml`, Rust 1.89.0,
    edition 2024, resolver 3).
  - Workspace-wide lint policy (`unsafe_code = deny`, `missing_docs`,
    `missing_debug_implementations`, `unreachable_pub`, `rust_2018_idioms`,
    `clippy::all`) and formatting config; enforced by CI.
  - `aetherc` driver crate: dependency-free CLI (`--help`, `--version`,
    compile-a-file) with proper, distinct exit codes. Cleanly reports the pipeline
    as unimplemented rather than faking success.
  - Tests: **13 passing** (7 unit for argument parsing + 6 integration exercising
    the built binary).
  - GitHub Actions CI (fmt · clippy · build · test), MIT license, `.gitignore`,
    `.gitattributes`.
  - The seven project-management documents authored and populated.

---

## Current progress

Milestone 0 is finished; the repository builds, lints cleanly, and its tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test` — 13 passed, 0 failed

---

## Next recommended milestone

**M1 — Source & diagnostics infrastructure.**

Introduce the `aether-source` and `aether-diagnostics` crates. Rationale: every
later phase (lexer onward) attaches spans to its output and reports errors, so
this foundation is a prerequisite and is reused everywhere.

Suggested scope:
- `aether-source`: `SourceMap` owning input files with stable `FileId`s; a compact
  `Span` (byte range + file); byte-offset → line/column resolution for
  diagnostics.
- `aether-diagnostics`: a structured `Diagnostic` (severity, message, primary and
  secondary labeled spans, notes); construction separated from rendering; a
  human-readable renderer that points into source with carets.
- Unit tests for span/line-column mapping and diagnostic rendering (including
  multi-byte UTF-8 handling).

Follow the standard workflow: review theory and alternatives, record decisions,
plan, then implement — and leave the repository green with updated docs.

---

## Architecture health

**Green.** The architecture is nascent but coherent: a documented pipeline, a
documented (incrementally materialized) crate graph, and enforced boundaries and
standards. No cycles, no premature abstractions, no placeholder crates.

---

## Outstanding work / technical debt

Nothing blocking. Known simplifications and future migrations are tracked in
[`TECH_DEBT.md`](TECH_DEBT.md) (notably: hand-rolled CLI → `clap` when justified;
`aetherc` pipeline is a stub by design until Phase 1 lands).
