# Aether — Roadmap

The long-term plan, organized into **phases** (themes) made of **milestones**
(one focused unit of work each — the intended scope of a single working session).
Milestones are deliberately sized so the repository is left in a coherent,
building, tested state after each one.

This roadmap is a living document. Milestone numbers are stable once assigned;
scope may be refined as we learn. Status legend: ✅ done · 🚧 in progress ·
⬜ planned · 🔭 future / not yet scheduled.

---

## Phase 0 — Foundation

Establish the platform's skeleton, tooling, and standards.

- **M0 — Project foundation** ✅
  Workspace, pinned toolchain, lint policy, formatting, CI, license, the seven
  project documents, and the `aetherc` driver skeleton (building + tested).

---

## Phase 1 — First Light *(minimal end-to-end pipeline)* — ✅ complete

Goal (achieved): compile and run a trivial program end to end, e.g.
`fn main() -> int { return 1 + 2; }` yielding `3`, via the AIR interpreter. The
whole architecture (lexer → parser → lowering → AIR → interpreter) is now
validated against real programs.

- **M1 — Source & diagnostics infrastructure** ✅
  `aether-source` (`SourceMap`, `Span`, byte→line/column mapping) and
  `aether-diagnostics` (structured diagnostics + caret rendering). Reused by every
  later phase, so it came first.
- **M2 — Lexer** ✅
  `aether-lexer`: tokenizes the minimal language into a spanned token stream
  (payload-free `Copy` tokens) with lexical error recovery through diagnostics;
  `aetherc` gained a `--dump-tokens` flag.
- **M3 — AST & parser** ✅
  `aether-ast` (`Box`-owned spanned tree + pretty-printer) and `aether-parser`
  (recursive descent with Pratt expression parsing, error recovery) for the
  minimal grammar; `aetherc` gained a `--dump-ast` flag.
- **M4 — AIR core & lowering** ✅
  `aether-air` (typed, SSA, id/arena IR + textual printer + verifier) and
  `aether-lower` (AST → AIR); ratified the AIR design (ADR-0013, supersedes
  ADR-0006); `aetherc` gained a `--dump-air` flag.
- **M5 — AIR interpreter** ✅
  `aether-air-interp`: executes AIR and produces the program's result (wrapping
  arithmetic; division by zero is a runtime error — ADR-0014/0015). **First
  runnable end-to-end pipeline** — `aetherc file.ae` runs a program and prints
  `main`'s result.

---

## Phase 2 — Language & Frontend Depth

Grow the language and give the frontend real semantic teeth.

- **M6 — Language expansion** 🚧 in progress
  Grows the language beyond a single `return`. Tackled in slices across sessions,
  in dependency order:
  1. **Local variables & bindings** ✅ — `let`, identifier expressions, and a
     name→value environment in lowering (straight-line, single-block AIR; lowering
     does provisional name resolution — ADR-0016).
  2. **Control flow** ⬜ ← next — `if`/`else`, comparison & boolean operators.
     Introduces multi-block AIR, branch terminators, and SSA merges — where the
     phi-node vs. block-parameter choice is decided (TD-0019) — plus CFG execution
     in the interpreter.
  3. **Functions: parameters & calls** ⬜ — adds a `:` token, parameters, a call
     instruction, and interpreter call frames.
- **M7 — Name resolution & scopes** ⬜
  Resolve identifiers to bindings; scope and shadowing rules.
- **M8 — Type system & checking** ⬜
  `aether-sema`: a checkable type system beyond `int`, with clear type-error
  diagnostics.

---

## Phase 3 — Optimization & Analysis Frameworks

Turn the middle end into a reusable optimization platform.

- **M9 — Pass & analysis framework** ⬜
  A pass manager (ordering, invalidation) and an analysis framework with cached,
  dependency-tracked results.
- **M10 — Core analyses** ⬜
  Control-flow graph, dominator tree, liveness, and a reusable dataflow engine.
- **M11 — Core optimizations** ⬜
  Constant folding/propagation, dead-code elimination, and simple CFG
  simplification, each verified against AIR invariants.

---

## Phase 4 — Native Backend

Emit real machine code without coupling earlier stages to any target.

- **M12 — Codegen framework & target abstraction** ⬜
  Backend interface, instruction-selection scaffolding, target description.
- **M13 — First native target (x86-64)** ⬜
  Lower AIR to x86-64; produce runnable native output.
- **M14 — Register allocation & ABI** ⬜
  A real register allocator and calling-convention handling.

---

## Phase 5 — Tooling & Platform

Make the platform pleasant to develop and to use.

- **M15 — Visualization** ⬜
  AST / AIR / CFG dumping and graph export (e.g. DOT).
- **M16 — Benchmarking & statistics** ⬜
  A benchmark harness and compiler statistics/timing, feeding `BENCHMARKS.md`.

---

## Phase 6 — Ecosystem *(future vision, not yet scheduled)* 🔭

Directions the architecture is meant to accommodate later, each a project in its
own right: standard library, package manager, formatter, linter, Language Server
Protocol implementation, documentation generator, build system, JIT, incremental
and parallel compilation, and eventually self-hosting.

These are **not** to be stubbed out today. They inform today's seams; they do not
add today's code.

---

## Next milestone

**M6 — Language expansion**, next slice: control flow (`if`/`else`). See
[`PROJECT_STATUS.md`](PROJECT_STATUS.md).
