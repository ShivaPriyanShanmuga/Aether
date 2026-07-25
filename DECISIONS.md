# Aether — Architecture Decision Records

This file records significant architectural decisions as **ADRs**. Each entry
captures the context, the alternatives weighed, the decision, and its
consequences, so that future contributors understand *why* the system is the way
it is — and can revisit a decision deliberately rather than by accident.

ADRs are append-only and immutable once **Accepted**. To reverse one, add a new
ADR that supersedes it and update the older entry's status. Status values:
Proposed · Accepted · Superseded · Deprecated.

---

## ADR-0001 — Implement Aether in Rust

**Status:** Accepted

**Context.** Aether is a long-lived, correctness- and performance-sensitive
compiler platform expected to exceed 100k lines and attract contributors.

**Alternatives considered.**
- **C++** — closest to LLVM/GCC/MLIR with vast prior art; but manual memory
  management, slower iteration, and a weaker default build/tooling story.
- **Zig** — modern and simple with fine control; but a young ecosystem and
  unproven at large-compiler scale.
- **Go** — fast builds and easy concurrency; but GC and weak sum types make
  expressing rich ASTs/IR awkward.

**Decision.** Use **Rust**.

**Rationale.** Algebraic data types with exhaustive pattern matching map directly
onto ASTs and IR nodes; memory safety without a GC suits a performance-focused
platform; Cargo workspaces give clean, independently testable module boundaries;
and Rust is *proven at exactly this task* (rustc, Cranelift, rust-analyzer).

**Consequences.** Rust's ownership model shapes IR data-structure design (arenas,
indices, and interners rather than pervasive pointers). Excellent built-in
tooling (cargo, clippy, rustfmt, test harness) is available from day one.

---

## ADR-0002 — Compile a custom minimal source language

**Status:** Accepted

**Context.** A compiler platform needs an input language.

**Alternatives considered.**
- **C subset** — well-specified semantics and abundant real test programs; but
  the preprocessor, undefined behavior, and type system make the early frontend
  far heavier.
- **Start from textual AIR** — defer the frontend and build the middle/back end
  first; faster to codegen work but a less complete-compiler experience early.

**Decision.** Design a **small, growable custom language** (source extension
`.ae`), beginning with integers, functions, and control flow.

**Rationale.** Full control over semantics keeps the early frontend tractable and
aligns with the long-term ecosystem vision (own language, standard library, and
tooling). We can grow the language exactly in step with the compiler's needs.

**Consequences.** We own language design decisions and must specify semantics as
we go. The language will start deliberately tiny to enable an end-to-end slice.

---

## ADR-0003 — Build a thin vertical slice first

**Status:** Accepted

**Context.** The platform has many subsystems (frontend, IR, analyses,
optimizations, backends). They can be built breadth-first (frameworks first),
depth-first (finish the frontend first), or as a thin end-to-end slice.

**Decision.** Build a **thin vertical slice** — a trivial program compiled and run
end to end — before deepening any single subsystem.

**Rationale.** An end-to-end path validates the architecture against a real
consumer early, surfaces interface problems between phases while they are cheap to
fix, and produces a runnable artifact that every later milestone extends.

**Consequences.** Early subsystems are intentionally minimal and will be revisited
and hardened in later phases; each such simplification is tracked in
[`TECH_DEBT.md`](TECH_DEBT.md).

---

## ADR-0004 — First execution target is an AIR interpreter

**Status:** Accepted

**Context.** The first vertical slice needs a way to actually run programs.

**Alternatives considered.**
- **Native x86-64 directly** — most impressive result; but register allocation
  and ABI details make early progress slow and error-prone.
- **WebAssembly** — clean, portable target, but still real codegen work.
- **LLVM IR backend** — fastest path to optimized native binaries, but a heavy
  external dependency that weakens the "our own backend" story.

**Decision.** Implement an **AIR interpreter** as the first backend.

**Rationale.** An interpreter validates AIR's semantics and, later, the
correctness of optimization passes without the complexity of codegen. It is the
fastest route to a correct, fully testable pipeline. A native backend follows in
Phase 4, once AIR is proven.

**Consequences.** The interpreter is a permanent asset — a semantic reference and
a differential-testing oracle for native backends — not throwaway work.

---

## ADR-0005 — Cargo workspace with per-subsystem crates

**Status:** Accepted

**Context.** The codebase must stay modular, loosely coupled, and independently
testable as it grows past 100k lines.

**Decision.** Use a **Cargo workspace**; each subsystem is its own crate, with
dependencies flowing frontend → IR → backend and foundational crates at the
bottom (see [`ARCHITECTURE.md`](ARCHITECTURE.md) §3). Shared version, edition, and
lint policy live at the workspace root.

**Rationale.** Crate boundaries enforce architectural boundaries at compile time:
a cycle or an illegal dependency simply will not compile. Workspace-level lints
apply one standard everywhere.

**Consequences.** Crates are created **only when a real consumer needs them** — no
empty placeholder crates. The workspace begins with a single member (`aetherc`)
and grows one milestone at a time.

---

## ADR-0006 — AIR is a typed, SSA-based, textual, verifiable IR

**Status:** Proposed *(direction only; to be ratified in the AIR milestone, M4)*

**Context.** AIR is the reusable core of the platform; its shape strongly affects
how easy analyses, optimizations, and backends are to write.

**Direction.** AIR is intended to be typed, SSA-based over a CFG of basic blocks,
round-trippable through a textual form, and checked by a verifier. Alternatives
(e.g. a non-SSA IR with explicit variables) are simpler up front but weaker for
optimization.

**Why record it now.** So earlier milestones aim at a consistent target. The
**detailed** design (instruction set, type system, textual grammar) is
intentionally deferred to M4 to avoid over-committing before we have a real
lowering and interpreter informing it.

**Consequences.** This ADR will be replaced by an **Accepted** ADR once the AIR
design is finalized against a working lowering and interpreter.

---

## ADR-0007 — Dependency-minimal foundation

**Status:** Accepted

**Context.** Early dependencies are easy to add and hard to remove, and each one
is a maintenance and supply-chain commitment.

**Decision.** Keep the foundation **dependency-free**: edition 2024, a pinned
stable toolchain, and a hand-rolled CLI in `aetherc` for now. Third-party crates
are added only when they clearly earn their place.

**Rationale.** A trivially small CLI does not justify a dependency yet. Staying
lean keeps builds fast and the trust surface small while the design settles.

**Consequences.** When the CLI surface grows (subcommands, many flags), we will
migrate argument parsing to `clap` — tracked in [`TECH_DEBT.md`](TECH_DEBT.md).
The MIT license was chosen for simplicity; dual MIT/Apache-2.0 (the Rust-ecosystem
norm) remains an easy future option.

---

## ADR-0008 — Source positions are per-file byte-range spans

**Status:** Accepted

**Context.** Every token, AST node, and IR value must reference the source that
produced it, so `Span` is attached pervasively; its size and interpretation cost
matter.

**Alternatives considered.**
- **Line/column pairs on each node** — simple to read, but fat (a range is four
  integers), invalidated by edits, and awkward to merge. Rejected by essentially
  all production compilers.
- **Global concatenated byte offsets** (rustc) — a single `u32` identifies both
  file and offset across one address space, packable into 4 bytes. Very compact,
  but requires a central concatenation and is less obvious.

**Decision.** Represent a span as `Span { file: FileId, lo: BytePos, hi: BytePos }`
— per-file byte offsets. Line/column is computed on demand from a per-file line
table (binary search). Columns count Unicode scalar values.

**Rationale.** Byte offsets keep `Span` small and `Copy` while mapping directly
onto source slices; computing line/column lazily keeps the common path cheap. The
explicit `FileId` makes spans self-describing and the model modular (each file is
self-contained), which is clearer than a global address space. Crucially, `Span`'s
fields are **private behind accessors**, so the representation can later adopt
rustc-style packing without touching any caller.

**Consequences.** A span is 12 bytes today rather than the 4-8 a packed scheme
would use; packing is deferred until profiling justifies it (TECH_DEBT.md
TD-0006). Column display width (CJK, tabs) is approximated as one column per
character for now (TD-0008).

---

## ADR-0009 — Diagnostics separate construction, collection, and rendering

**Status:** Accepted

**Context.** Diagnostics are produced by every phase and are central to a
compiler's usability. How they are built, gathered, and presented shapes both
ergonomics and testability.

**Decision.** Split the concern three ways:
- **Construction** — an immutable-style fluent builder produces a structured
  `Diagnostic` (severity, optional code, primary/secondary labeled spans, notes).
- **Collection** — phases emit into a `DiagnosticHandler` that buffers diagnostics
  and tracks error/warning counts.
- **Rendering** — a standalone `render(&Diagnostic, &SourceMap) -> String`
  produces human-readable, caret-annotated plain text.

**Rationale.** Buffering (rather than printing at the emit site) gives the driver
control over ordering and over whether to proceed after errors, and makes
diagnostics assertable as data in tests. Keeping rendering separate lets the
presentation evolve — color, JSON for IDEs, alternative formats — without touching
the phases that emit diagnostics.

**Consequences.** For now rendering is plain text with single-line underlines;
color output (TD-0009) and richer multi-line/label-grouped rendering (TD-0007) are
deferred. `aether-diagnostics` depends on `aether-source` (to resolve spans),
consistent with the dependency rules in `ARCHITECTURE.md`.

---

## ADR-0010 — Tokens are payload-free; string interning is deferred

**Status:** Accepted

**Context.** The lexer's output representation shapes the parser and, indirectly,
name resolution. A recurring question is whether tokens should carry their text or
value, and whether to introduce string interning (and a `Session` to own the
interner) now.

**Alternatives considered.**
- **Tokens carry owned data** (`Ident(String)`, `Int(u64)`) — self-contained, but
  tokens become non-`Copy` and allocate, and value parsing/overflow handling gets
  baked into the lexer.
- **Tokens carry an interned `Symbol`** (rustc) — fast identifier comparison, but
  requires standing up an interner and a `Session` immediately, before any
  consumer needs fast symbol comparison.
- **Payload-free tokens** — `TokenKind` is a `Copy` enum; the lexeme is recovered
  from the source via the token's span.

**Decision.** Use **payload-free tokens**. `TokenKind` carries no text or value;
identifier text and integer values are recovered from the source via the span.

**Rationale.** Tokens stay tiny and `Copy` (trivial to peek/clone in the parser),
and a token's *shape* is cleanly separated from its *value*. Crucially, the lexer
then needs **no string interning**, so the interner and the `Session` type can be
deferred until name resolution (M7) actually benefits from fast symbol comparison
— avoiding a premature abstraction (TD-0010).

**Consequences.** The parser recovers identifier text and parses integer values
from span text (via the `SourceMap`). Integer-literal validation (overflow, bases,
underscores) therefore happens at or after parsing, not during lexing (TD-0012).
When interning is introduced, it will likely arrive together with the `Session`.
