# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 1 — First Light
**Current milestone:** M3 — AST & parser → ✅ **complete**
**Next milestone:** M4 — AIR core & lowering

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Completed milestones

- **M3 — AST & parser** ✅
  - `aether-ast`: `Box`-owned, self-contained node tree (`Program`, `Item`,
    `FnDecl`, `Type`, `Block`, `Stmt`, `ReturnStmt`, `Expr`, `BinOp`, `UnOp`,
    `Ident`), every node spanned; identifiers store their text and integer
    literals their parsed value. Plus a `pretty` tree-printer (needs no source
    map) used for golden tests.
  - `aether-parser`: hand-written recursive descent with **Pratt** expression
    parsing (binding-power table). Correct precedence/associativity, unary minus,
    parentheses. Error-tolerant: poison `Expr::Error` nodes and
    synchronize-to-next-`fn` recovery; integer-overflow diagnostics.
  - `aetherc` now parses after a clean lex and gained a `--dump-ast` flag; syntax
    errors render as caret diagnostics and exit `1`.
  - Two ADRs recorded (ADR-0011: AST uses `Box`; ADR-0012: Pratt expression
    parser).
  - Tests: **85 total, all passing** (+2 ast, +12 parser incl. doctest, +3 driver).
- **M2 — Lexer** ✅
  - `aether-lexer`: payload-free `Copy` tokens, hand-written scanner, error
    recovery, `--dump-tokens`.
- **M1 — Source & diagnostics infrastructure** ✅
  - `aether-source` (spans, `SourceMap`, line/column) and `aether-diagnostics`
    (structured diagnostics + caret rendering).
- **M0 — Project foundation** ✅
  - Cargo workspace, pinned toolchain, lint policy, CI, MIT license, driver
    skeleton, and the seven project-management documents.

---

## Current progress

Milestone 3 is finished; the workspace builds, lints cleanly, and all tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 85 passed, 0 failed

The frontend, exercised through the binary:
`aetherc --dump-ast file.ae` prints the parsed tree; a syntax error renders caret
diagnostics and exits `1`. `--dump-tokens` still stops after lexing.

---

## Next recommended milestone

**M4 — AIR core & lowering.**

Introduce `aether-air` — the compiler's own intermediate representation — and
lower the AST into it. Rationale: AIR is the reusable heart of the platform (the
target for analyses, optimizations, and every backend). Getting a minimal,
well-designed AIR and an AST→AIR lowering in place is the step that makes the
first runnable program (M5, the interpreter) possible.

Suggested scope:
- **Ratify the AIR design** (this is the milestone ADR-0006 was deferred to):
  typed, SSA-based over a CFG of basic blocks, with a textual form and a verifier.
  Decide the concrete representation — almost certainly index/arena-based (values,
  blocks, instructions referenced by ids), the counterpart to the AST's `Box`
  tree (ADR-0011). Record the finalized design as an ADR superseding ADR-0006.
- **Minimal instruction set** sufficient for the current language: integer
  constants, the arithmetic ops (`add`/`sub`/`mul`/`div`, plus negation), and a
  `ret`. One function, one or few blocks.
- **Lowering** `aether-ast` → `aether-air` for the minimal program.
- **Textual printer** (for golden tests and a `--dump-air` flag) and a **verifier**
  checking structural invariants (e.g. every value defined before use, block
  terminators well-formed).
- Tests: lowering golden tests (AST → printed AIR), verifier tests.

Open decision to weigh at the start: whether M4 finally introduces `aether-support`
(arena/id primitives) and/or the `Session` type, since AIR's id-based design is
the first real consumer of arena/interning-style infrastructure (TD-0010).

Follow the standard workflow: review theory and alternatives, record decisions,
plan, then implement — and leave the repository green with updated docs.

---

## Architecture health

**Green.** The frontend is complete through parsing, with clean one-directional
dependencies: `aether-source` (no deps) ← `aether-diagnostics` ← `aether-lexer`;
`aether-ast` ← `aether-source`; `aether-parser` ← {ast, lexer, source,
diagnostics}; the driver on top. No cycles, no premature abstractions, no
placeholder crates. The AST↔IR representation split (Box tree vs. forthcoming
id-based AIR) is a deliberate, documented boundary. `Session`/interning still
deferred (TD-0010), to be reconsidered when AIR lands.

---

## Outstanding work / technical debt

Nothing blocking. Tracked in [`TECH_DEBT.md`](TECH_DEBT.md): from M3 — basic
parser recovery (TD-0016), no parser recursion-depth guard (TD-0017), no function
parameters yet (TD-0018); plus carry-overs — lexer limits (TD-0011…0015), `Span`
packing (TD-0006), diagnostic polish (TD-0007/8/9), deferred `Session` (TD-0010),
and the hand-rolled CLI → `clap` migration (TD-0001), increasingly worth doing as
`--dump-*` flags accumulate.
