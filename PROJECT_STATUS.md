# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 1 — First Light → ✅ **complete**
**Current milestone:** M5 — AIR interpreter → ✅ **complete**
**Next milestone:** M6 — Language expansion (starting with local variables)

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Milestone

**Phase 1 (First Light) is complete.** A source program now compiles and runs end
to end — the entire architecture (lexer → parser → lowering → AIR → interpreter)
is validated against real programs. `aetherc file.ae` runs a program and prints
the value returned by `main`.

---

## Completed milestones

- **M5 — AIR interpreter** ✅
  - `aether-air-interp`: a tree-walking interpreter over AIR. Evaluates a
    function's instructions (SSA values → `i64`) and acts on the terminator.
    Wrapping arithmetic; division by zero is a runtime error carrying a span
    (ADR-0015). Decoupled from diagnostics (returns a `RunError`).
  - `aetherc` now **executes** by default: full pipeline (lex → parse → lower →
    verify → interpret), printing `main`'s result to stdout (ADR-0014). New exit
    code `RUNTIME_ERROR` (70); the old "unimplemented" path (exit 3) is gone.
  - Two ADRs (ADR-0014 result surfacing; ADR-0015 arithmetic semantics).
  - Tests: **111 total, all passing** (+9 interp incl. doctest, +2 driver net).
- **M4 — AIR core & lowering** ✅
  - `aether-air` (typed, SSA, id/arena IR + printer + verifier) and `aether-lower`
    (AST → AIR); ratified AIR in ADR-0013.
- **M3 — AST & parser** ✅
  - `aether-ast` (Box tree + pretty-printer) and `aether-parser` (recursive
    descent + Pratt, error recovery); `--dump-ast`.
- **M2 — Lexer** ✅
  - `aether-lexer`: payload-free `Copy` tokens, error recovery; `--dump-tokens`.
- **M1 — Source & diagnostics infrastructure** ✅
  - `aether-source` and `aether-diagnostics` (structured diagnostics + caret
    rendering).
- **M0 — Project foundation** ✅
  - Cargo workspace, pinned toolchain, lint policy, CI, MIT license, driver
    skeleton, and the seven project-management documents.

---

## Current progress

Milestone 5 is finished; the workspace builds, lints cleanly, and all tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 111 passed, 0 failed

End-to-end, through the binary: `aetherc file.ae` for
`fn main() -> int { return (10 - 4) * 7 + -2; }` prints `40` and exits `0`;
`10 / (5 - 5)` prints a caret diagnostic "division by zero" and exits `70`. The
`--dump-tokens` / `--dump-ast` / `--dump-air` flags stop after their phases.

---

## Next recommended milestone

**M6 — Language expansion**, beginning with its first slice: **local variables &
bindings**.

Rationale: Phase 1 proved the pipeline on a single `return`. The next depth is a
real language. Local variables are the right first slice because they add genuine
expressiveness while staying **straight-line** — no control flow, so AIR remains
single-block and no SSA merges (phi/block-parameters) are needed yet. This keeps
the milestone focused and defers the harder CFG work to the control-flow slice.

Suggested scope (slice 1 — local variables):
- Lexer: add the `let` keyword and the `=` token.
- Grammar/AST: a `let NAME = <expr>;` statement, a sequence of statements before
  the `return`, and an identifier expression (`Expr::Path`/`Name`) that refers to
  a binding.
- Lowering: maintain a name → `Value` environment; `let` binds a name to the
  value of its initializer; an identifier expression resolves to that value. Still
  one block. (This is trivial SSA — no phi — because there is no control flow.)
- Decide how an unknown identifier is reported (a lowering/resolution diagnostic
  now, or deferred to the M9 name-resolution pass) — record the choice.
- Reassess whether interning / a `Session` is now justified (TD-0010): a name
  environment is the first place symbol comparison happens, though a `String` or
  `&str` keyed map is fine at this scale.
- Tests: parser (let statements, identifier expressions, precedence unaffected),
  lowering golden tests (env resolves names to values), interpreter results, and a
  driver end-to-end test.

Later slices of M6 (separate sessions): **control flow** (`if`/`else`, comparison
& boolean operators; multi-block AIR + SSA merges + CFG execution — the point to
decide phi vs. block parameters, TD-0019/TD-0023) and **functions: parameters &
calls** (`:` token, params, a call instruction, interpreter call frames).

Follow the standard workflow: review theory and alternatives, record decisions,
plan, then implement — and leave the repository green with updated docs.

---

## Architecture health

**Green.** Eight crates with clean, one-directional dependencies:
`aether-source` (no deps) is the base; `aether-diagnostics`, `aether-lexer`,
`aether-ast`, `aether-air` build on it; `aether-parser` sits above the frontend;
`aether-lower` bridges AST → AIR; `aether-air-interp` executes AIR; the driver
orchestrates. No cycles, no premature abstractions, no placeholder crates. The
AST `Box` tree (ADR-0011) and AIR id/arena (ADR-0013) split is deliberate.
`aether-support` and a `Session` type are still unneeded and stay deferred
(TD-0010) — to be reconsidered when local variables introduce a name environment.

---

## Outstanding work / technical debt

Nothing blocking. Tracked in [`TECH_DEBT.md`](TECH_DEBT.md). Resolved in M5:
TD-0002 (pipeline stub). From M5: interpreter executes only the entry block
(TD-0023), runtime values are `i64` only (TD-0024), overflow policy is provisional
(TD-0025). Carry-overs: single-block AIR / no phi yet (TD-0019),
missing-return-as-verifier-error pending M8 (TD-0020), literal/type-range checks
deferred to M8 (TD-0021), no AIR text parser (TD-0022), parser
recovery/depth/params (TD-0016…0018), lexer limits (TD-0011…0015), `Span` packing
(TD-0006), diagnostic polish (TD-0007/8/9), deferred `Session` (TD-0010), and the
hand-rolled CLI → `clap` migration (TD-0001), now carrying four `--dump-*` flags
plus the default run behavior.
