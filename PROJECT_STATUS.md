# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 1 — First Light
**Current milestone:** M4 — AIR core & lowering → ✅ **complete**
**Next milestone:** M5 — AIR interpreter

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Completed milestones

- **M4 — AIR core & lowering** ✅
  - `aether-air`: the compiler's own IR — typed, SSA, id/arena representation
    (`Module` → `Function` → arenas of instructions addressed by `Value` and
    blocks addressed by `Block`; every value is an instruction result). Minimal
    instruction set (`iconst`, `add`/`sub`/`mul`/`div`, `neg`, `ret`), a textual
    printer, and a structural verifier (terminated blocks, def-before-use, type
    agreement, return-type match).
  - `aether-lower`: AST → AIR lowering (post-order, naturally SSA). Keeps AIR
    frontend-independent.
  - Ratified the AIR design in ADR-0013 (supersedes ADR-0006).
  - `aetherc` gained `--dump-air`; it now lowers + verifies after a clean parse,
    and reports verification failures.
  - Tests: **100 total, all passing** (+7 air, +5 lower incl. doctest, +3 driver).
- **M3 — AST & parser** ✅
  - `aether-ast` (`Box`-owned spanned tree + pretty-printer) and `aether-parser`
    (recursive descent + Pratt, error recovery); `--dump-ast`.
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

Milestone 4 is finished; the workspace builds, lints cleanly, and all tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 100 passed, 0 failed

The pipeline that exists today, exercised through the binary: `aetherc
--dump-air file.ae` lowers a program (source → tokens → AST → AIR) and prints the
IR; a function that never returns is caught by the verifier and exits `1`.
`--dump-tokens` / `--dump-ast` stop after their phases.

---

## Next recommended milestone

**M5 — AIR interpreter.**

Introduce `aether-air-interp`: execute an AIR module and produce the program's
result. Rationale: this closes the first end-to-end vertical slice — a source
program actually *runs* — validating the entire architecture (lexer → parser →
lower → AIR → execution) against real output, per the vertical-slice strategy
(ADR-0003) and the interpreter-first decision (ADR-0004).

Suggested scope:
- A tree/CFG-walking interpreter over AIR: evaluate each block's instructions in
  order into a value map (`Value` → runtime integer), follow the terminator
  (`Ret` yields the function's result). Single block today; the structure should
  anticipate branches.
- Define runtime semantics for the existing ops, including integer division:
  decide behavior for division by zero (e.g. a runtime error/diagnostic) and
  overflow (wrapping vs. checked) — record as an ADR.
- Wire into `aetherc`: run `main` and report its result (e.g. print it, and/or use
  it as the process exit code). Replace the "unimplemented beyond AIR" path with
  actual execution.
- Tests: interpreter unit tests over hand-built/lowered modules (arithmetic,
  precedence, negation), and an end-to-end driver test asserting a program's
  computed result.

Open decision to weigh: how a program's result is surfaced by the CLI (printed
value vs. process exit code vs. both), and the division-by-zero / overflow policy.

Follow the standard workflow: review theory and alternatives, record decisions,
plan, then implement — and leave the repository green with updated docs.

---

## Architecture health

**Green.** The frontend is complete through AIR lowering, with clean
one-directional dependencies: `aether-source` (no deps) is the base;
`aether-diagnostics`, `aether-lexer`, `aether-ast`, `aether-air` build on it;
`aether-parser` sits above the frontend crates; `aether-lower` bridges AST → AIR
without coupling AIR to the AST; the driver orchestrates. No cycles, no premature
abstractions, no placeholder crates. AIR's id/arena design (ADR-0013) is the
deliberate counterpart to the AST's `Box` tree (ADR-0011). `aether-support` and a
`Session` type remain unneeded so far and stay deferred (TD-0010).

---

## Outstanding work / technical debt

Nothing blocking. Tracked in [`TECH_DEBT.md`](TECH_DEBT.md): from M4 — AIR is
single-block with no phi/block-params yet (TD-0019), missing-return surfaces as a
verifier error pending semantic analysis (TD-0020), literal-range/type-name
checks deferred to M8 (TD-0021), no AIR textual parser (TD-0022); plus
carry-overs — parser recovery/depth/params (TD-0016…0018), lexer limits
(TD-0011…0015), `Span` packing (TD-0006), diagnostic polish (TD-0007/8/9),
deferred `Session` (TD-0010), and the hand-rolled CLI → `clap` migration
(TD-0001), now carrying four `--dump-*` flags.
