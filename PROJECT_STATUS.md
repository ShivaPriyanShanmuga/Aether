# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 1 — First Light
**Current milestone:** M2 — Lexer → ✅ **complete**
**Next milestone:** M3 — AST & parser

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Completed milestones

- **M2 — Lexer** ✅
  - `aether-lexer`: a hand-written, character-based scanner. Payload-free `Copy`
    `TokenKind` (text recovered from source via span), `Token { kind, span }`,
    and `tokenize(&SourceFile) -> LexResult { tokens, diagnostics }`.
  - Token set for the minimal language: identifiers, integer literals, keywords
    `fn`/`return`, delimiters `( ) { }`, punctuation `; ,`, operators
    `+ - * / ->`, line comments `//`, and an explicit `Eof`.
  - Error recovery: an unexpected character emits a `Diagnostic` and scanning
    continues. UTF-8-correct spans (multi-byte chars handled).
  - `aetherc` now lexes input, surfaces lexical errors (new exit code `1`), and
    has a `--dump-tokens` debug flag (first bit of compiler tooling).
  - One ADR recorded (ADR-0010: payload-free tokens; interning deferred).
  - Tests: **68 total, all passing** (+18 lexer unit + 1 lexer doctest, +3 driver).
- **M1 — Source & diagnostics infrastructure** ✅
  - `aether-source` (`BytePos`, `FileId`, `Span`, `LineCol`, `SourceFile`,
    `SourceMap`; byte→line/column via a line table; UTF-8/CRLF aware).
  - `aether-diagnostics` (structured `Diagnostic` + fluent builder,
    `DiagnosticHandler`, rustc-style caret `render`).
- **M0 — Project foundation** ✅
  - Cargo workspace, pinned toolchain (Rust 1.89.0, edition 2024, resolver 3),
    workspace lint policy, CI, MIT license, `aetherc` driver skeleton, and the
    seven project-management documents.

---

## Current progress

Milestone 2 is finished; the workspace builds, lints cleanly, and all tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 68 passed, 0 failed

The full pipeline that exists today, exercised through the binary:
`aetherc --dump-tokens file.ae` prints the token stream; an invalid character
renders a caret diagnostic and exits `1`.

---

## Next recommended milestone

**M3 — AST & parser.**

Introduce `aether-ast` (spanned node definitions) and `aether-parser` (a
recursive-descent parser with precedence-based expression parsing) for the
minimal grammar. Rationale: the parser is the natural consumer of the token
stream and produces the tree that AIR lowering (M4) will consume — the next link
toward the first runnable program.

Suggested scope:
- Grammar for the minimal language: `fn NAME(params) -> TYPE { stmts }`, a
  `return <expr>;` statement, and an expression grammar covering integer
  literals, identifiers, the binary operators `+ - * /` (with correct
  precedence/associativity), unary minus, and parenthesized grouping.
- `aether-ast`: node types (`Item`/`Fn`, `Stmt`, `Expr`, `Type`) each carrying a
  `Span`. Decide node ownership strategy (arena vs `Box`) and record it as an ADR.
- `aether-parser`: consumes `&[Token]` + the `SourceMap` (for lexeme text and
  literal values), produces the AST, and reports syntax errors as `Diagnostic`s
  with error recovery (e.g. synchronize on `;`/`}`).
- Open decision to settle at the start: whether the parser stores identifier text
  by span (continuing to defer interning) or introduces `aether-support` + a
  `Session` with a string interner now. Recommendation stands to defer until name
  resolution (M7) unless the parser reveals a concrete need.
- Tests: unit tests for each production, precedence/associativity, error recovery,
  and span correctness; consider a small AST pretty-printer for golden tests.

Follow the standard workflow: review theory and alternatives, record decisions,
plan, then implement — and leave the repository green with updated docs.

---

## Architecture health

**Green.** The frontend's first phases are in place with clean dependencies:
`aether-source` (no deps) ← `aether-diagnostics` ← `aether-lexer`; the driver sits
on top and wires them together. Boundaries hold, no cycles, no premature
abstractions, no placeholder crates. The `Session`/context type and string
interning remain deliberately deferred (TD-0010) — the payload-free token design
means nothing needs them yet.

---

## Outstanding work / technical debt

Nothing blocking. Tracked in [`TECH_DEBT.md`](TECH_DEBT.md): from M2 — ASCII-only
identifiers (TD-0011), integer-literal limitations (TD-0012), no block comments
(TD-0013), no error-code registry (TD-0014), and lexer peek re-slicing (TD-0015);
plus carry-overs — `Span` packing (TD-0006), diagnostic rendering polish
(TD-0007/8/9), the deferred `Session` (TD-0010), and the hand-rolled CLI →
`clap` migration (TD-0001), now mildly more pressing as flags accumulate.
