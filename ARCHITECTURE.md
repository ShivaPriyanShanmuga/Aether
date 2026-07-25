# Aether — Architecture

This document describes the high-level architecture of the Aether compiler
platform: its pipeline, its crate structure, the rules that keep it modular, and
the design principles that govern change. It is updated whenever the architecture
changes. For *why* specific choices were made, see [`DECISIONS.md`](DECISIONS.md).

---

## 1. Overview

Aether translates source text written in the (small, growing) Aether language
into an executable result. It is structured as a classic multi-phase compiler
with a strong, reusable middle end built around a custom intermediate
representation, **AIR (Aether Intermediate Representation)**.

The guiding architectural idea is **separation by phase and by crate**: each
stage of compilation is an independently testable library with a narrow,
well-documented interface, coordinated by a thin driver. No stage reaches across
another stage's internals; they communicate through explicit data structures
(token streams, ASTs, AIR modules).

---

## 2. Compilation pipeline

```
                 ┌─────────────────────────────────────────────────────────┐
                 │                    Compilation Session                   │
                 │   (config, source map, diagnostics sink, interners)      │
                 └─────────────────────────────────────────────────────────┘
                          │ cross-cutting context, threaded through phases
   source (.ae)           ▼
     │
     ▼
 ┌────────┐   tokens   ┌────────┐   AST    ┌──────────┐  typed AST  ┌──────────┐
 │ Lexer  │ ─────────▶ │ Parser │ ───────▶ │  Semantic │ ─────────▶ │   AIR    │
 └────────┘            └────────┘          │  Analysis │            │ Lowering │
                                           └──────────┘            └──────────┘
                                                                        │ AIR
                                                                        ▼
                                        ┌──────────────────────────────────────┐
                                        │      Middle end (over AIR modules)     │
                                        │  Analyses  ◀──▶  Optimization passes   │
                                        └──────────────────────────────────────┘
                                                                        │ optimized AIR
                                          ┌─────────────────────────────┴───────┐
                                          ▼                                       ▼
                                   ┌──────────────┐                       ┌──────────────┐
                                   │ AIR Interp.  │  (first target)       │ Native codegen│  (future)
                                   │  → result    │                       │  → machine code│
                                   └──────────────┘                       └──────────────┘
```

**Cross-cutting concerns** (diagnostics, source management, string/symbol
interning, and the compilation session that owns them) are available to every
phase and are *not* themselves phases.

---

## 3. Crate structure

Aether is a Cargo workspace. Each subsystem is its own crate so it can be built,
tested, and reasoned about in isolation. **Crates are materialized only when a
real consumer needs them** — we do not create empty placeholder crates. The table
below therefore distinguishes what exists today from what is planned.

| Crate | Responsibility | Status |
| --- | --- | --- |
| `aetherc` | Command-line driver; orchestrates phases | **exists** |
| `aether-source` | Source files, `Span`, byte↔line/column mapping | **exists** |
| `aether-diagnostics` | Diagnostics: errors/warnings, caret rendering | **exists** |
| `aether-support` | Shared primitives: arenas, interners, data structures | planned (as needed) |
| `aether-lexer` | Lexical analysis → token stream | **exists** |
| `aether-ast` | AST node definitions (+ pretty-printer) | **exists** |
| `aether-parser` | Recursive-descent parser → AST | **exists** |
| `aether-air` | AIR data structures, builder, verifier, textual printer | **exists** |
| `aether-lower` | AST → AIR lowering (+ provisional name resolution) | **exists** |
| `aether-air-interp` | AIR interpreter (first execution target) | **exists** |
| `aether-sema` | Name resolution and type checking | planned (Phase 2) |
| `aether-analysis` | Analysis framework (CFG, dominators, dataflow, …) | planned (Phase 3) |
| `aether-opt` | Pass manager and optimization passes | planned (Phase 3) |
| `aether-codegen` | Backend framework and native targets | planned (Phase 4) |

Some cross-cutting crates (`aether-source`, `aether-diagnostics`, `aether-support`)
may begin life merged and split apart once their surface justifies it; the split
above is the intended end state, not a mandate to create thin crates early.

### Dependency rules

To keep the graph acyclic and the coupling loose, dependencies flow in one
direction:

1. **Foundational crates depend on nothing project-specific.** `aether-support`,
   `aether-source`, and `aether-diagnostics` sit at the bottom and may be used by
   anyone.
2. **Frontend → IR → backend, never backwards.** The parser does not know about
   AIR; AIR does not know about any backend; a backend does not know about the
   parser. Earlier stages must never depend on later ones.
3. **The driver (`aetherc`) sits on top** and is the only crate allowed to know
   about all phases, because its job is to connect them.
4. **No cycles, ever.** If two crates seem to need each other, the shared concept
   belongs in a lower foundational crate.

---

## 4. Cross-cutting components

- **Compilation Session / Context.** A single owner for per-compilation state:
  configuration, the source map, the diagnostics sink, and interners. Threaded
  explicitly through phases (no global mutable state), which keeps the compiler
  testable and, eventually, safe to parallelize. *Not yet materialized as a type:*
  with only a `SourceMap` and a `DiagnosticHandler` today, the driver holds them
  directly. The `Session` type is introduced once interners and multiple phases
  make bundling worthwhile (TECH_DEBT.md TD-0010) — deferring it avoids a
  premature abstraction.
- **Diagnostics.** A structured diagnostics engine (severity, primary/secondary
  spans, notes, suggestions) with rendering separated from construction. Phases
  *emit* diagnostics into the session rather than printing directly.
- **Source management.** A source map assigns stable identifiers to files and
  maps byte offsets (`Span`s) to line/column positions for diagnostics. Spans are
  carried on tokens and AST nodes from the very first phase.

---

## 5. AIR

AIR is the reusable heart of the platform. Its design was ratified in M4
(ADR-0013, superseding the ADR-0006 direction) and a **minimal implementation
exists**. The design is:

- **Typed** — every value has an AIR type; the IR is checkable. (Today the types
  are `int` and `bool`; comparisons produce `bool`.)
- **SSA-based** with a control-flow graph of basic blocks. Every value is the
  result of an instruction (constants included), so operands are uniformly other
  values. (Today functions have a single block. Multiple blocks and SSA merges
  arrive with control flow; the merge representation is decided — **block
  parameters**, not phi nodes, ADR-0017 — and lands in M6 slice 2b, at which point
  a value becomes "an instruction result *or* a block parameter".)
- **id/arena representation** — a `Function` owns flat arenas of instructions
  (addressed by `Value`) and blocks (addressed by `Block`), the deliberate
  counterpart to the AST's `Box` tree (ADR-0011). This gives stable ids, side
  tables, and cross-block references without lifetime threading.
- **Textual form** — a human-readable printer (used by `--dump-air` and golden
  tests). A textual *parser* (full round-tripping) is future work.
- **Verifiable** — a verifier checks structural invariants (every block
  terminated, def-before-use, per-instruction operand/result type agreement across
  `int`/`bool`, return type match). Dominance-based checking arrives with control
  flow.
- **Target-independent** and **frontend-independent** — `aether-air` depends only
  on `aether-source`; AST → AIR lowering lives in the separate `aether-lower`
  crate.

See [`DECISIONS.md`](DECISIONS.md) ADR-0013 for the ratified design.

---

## 6. Design principles

- **Architecture before implementation.** Design, weigh alternatives, then build.
- **No premature abstraction.** Introduce a crate, trait, or generalization only
  when a concrete second use or clear need justifies it.
- **Everything is testable in isolation.** Each phase takes explicit inputs and
  produces explicit outputs; no hidden global state.
- **Documentation is a deliverable.** Public items are documented (`missing_docs`
  is enforced); design rationale lives in these documents.
- **Measure before optimizing.** Performance work is guided by benchmarks
  (see [`BENCHMARKS.md`](BENCHMARKS.md)), not guesses.
- **The repository is the source of truth.** These documents, not any external
  memory, define the current state and plan.

---

## 7. Extension points (future)

The architecture is intended to absorb, without rewrites: additional language
features (frontend), additional analyses and optimization passes (middle end),
additional backends/targets (backend), and developer tooling (visualizers,
statistics). Each is an addition at an established seam, not a structural change.
