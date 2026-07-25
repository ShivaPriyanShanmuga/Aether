# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 1 — First Light
**Current milestone:** M1 — Source & diagnostics infrastructure → ✅ **complete**
**Next milestone:** M2 — Lexer

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Completed milestones

- **M1 — Source & diagnostics infrastructure** ✅
  - `aether-source`: `BytePos`, `FileId`, `Span` (compact, `Copy`, private
    representation behind accessors), `LineCol`, `SourceFile`, and `SourceMap`.
    Byte-offset → 1-based line/column resolution via a precomputed line table
    (binary search); UTF-8-aware column counting; CRLF-aware line text.
  - `aether-diagnostics`: structured `Diagnostic` (severity, optional code,
    primary/secondary labeled spans, notes) built with a fluent builder; a
    `DiagnosticHandler` that buffers and counts; and a `render` function producing
    rustc-style, caret-annotated, tab/UTF-8-aware plain-text output.
  - `aetherc` now loads input into a real `SourceMap` and reports the
    (still-unimplemented) pipeline through the real diagnostics renderer.
  - Tests: **47 passing** (17 `aether-source`, 17 `aether-diagnostics` incl. 7
    golden render tests + 1 doctest, 13 `aetherc`).
  - Two ADRs recorded (span representation; diagnostics architecture).
- **M0 — Project foundation** ✅
  - Cargo workspace, pinned toolchain (Rust 1.89.0, edition 2024, resolver 3),
    workspace-wide lint policy and formatting, GitHub Actions CI, MIT license.
  - `aetherc` driver skeleton with a dependency-free CLI and distinct exit codes.
  - The seven project-management documents authored and populated.

---

## Current progress

Milestone 1 is finished; the workspace builds, lints cleanly, and all tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 47 passed, 0 failed

---

## Next recommended milestone

**M2 — Lexer.**

Introduce the `aether-lexer` crate: tokenize the minimal language into a spanned
token stream. Rationale: with source and diagnostics in place, the lexer is the
first real frontend phase and the first genuine consumer of both — every token
carries a `Span`, and lexical errors are reported as `Diagnostic`s.

Suggested scope:
- Define the initial token set for the minimal language: identifiers, integer
  literals, keywords (`fn`, `return`, and the `int` type to start), punctuation
  and operators (`( ) { } ; , + - * / ->`), and end-of-file.
- A `Token { kind, span }` model and a `Lexer` that turns `&SourceFile` (or
  `&str` + `FileId`) into a token stream, skipping whitespace and comments.
- Lexical error recovery: on an unexpected character, emit a `Diagnostic` and
  continue, rather than aborting.
- Likely introduce `aether-support` (or fold into the lexer for now) if string
  interning for identifiers is warranted — decide when the need is concrete.
- Unit tests for each token kind, span correctness, and error recovery; consider
  a small snapshot/golden approach for token streams.

Follow the standard workflow: review theory and alternatives, record decisions,
plan, then implement — and leave the repository green with updated docs.

---

## Architecture health

**Green.** The pipeline's foundational layer is in place and cleanly separated:
`aether-source` depends on nothing; `aether-diagnostics` depends only on
`aether-source`; the driver sits on top. Boundaries hold, no cycles, no premature
abstractions, no placeholder crates. A `Session`/context type is deliberately
still deferred (see TECH_DEBT.md TD-0010) until interners and multiple phases
justify it.

---

## Outstanding work / technical debt

Nothing blocking. Known simplifications and future refinements are tracked in
[`TECH_DEBT.md`](TECH_DEBT.md) — notably `Span` packing (TD-0006), multi-line
diagnostic rendering (TD-0007), display-width columns (TD-0008), colored output
(TD-0009), and the deferred `Session` type (TD-0010), plus the carry-overs from
M0 (hand-rolled CLI → `clap`; `aetherc` pipeline still a stub until Phase 1
completes).
