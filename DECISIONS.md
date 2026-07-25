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

**Status:** Superseded by [ADR-0013](#adr-0013--air-ratified-design) *(the direction
recorded here was ratified and made concrete in M4)*

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

---

## ADR-0011 — The AST is a `Box`-owned tree

**Status:** Accepted

**Context.** The AST is the parser's output and AIR lowering's input. Its
representation affects parser ergonomics and how the AST is traversed.

**Alternatives considered.**
- **Arena allocation** (`&'arena Expr`) — fast, cache-friendly, no per-node drop,
  but threads an `'arena` lifetime through every AST-touching signature.
- **Index-based** (`ExprId` into a flat `Vec`) — cache-friendly, no lifetimes,
  enables side tables, but adds indirection and makes pattern matching clumsier.
- **`Box`-owned tree** — interior nodes own children via `Box`.

**Decision.** Use a **`Box`-owned tree**.

**Rationale.** The AST is a *transient* frontend product that is lowered to AIR
and discarded; it is not the long-lived optimization IR. `Box` is idiomatic,
dependency-free, and keeps recursive construction and pattern matching clean —
the right trade for the AST's role. The heavier index/arena machinery is reserved
for **AIR** (ADR-0011's counterpart, to be decided in M4), where SSA, side
tables, and in-place mutation actually need it.

**Consequences.** AST nodes are self-contained (identifiers store their text,
integer literals their value), so tooling like the pretty-printer needs no source
map. Very deeply nested expressions could, in theory, stress the stack on
construction/drop; not a concern at realistic scale, and a parser recursion-depth
guard is tracked separately (TD-0017).

---

## ADR-0012 — Expressions use a Pratt (precedence-climbing) parser

**Status:** Accepted

**Context.** The expression grammar has operator precedence and associativity and
will grow (comparison, logical, assignment, …). The parser is otherwise
straightforward recursive descent.

**Alternatives considered.**
- **One function per precedence level** (`parse_add`, `parse_mul`, …) — explicit
  and readable, but adds a function per level and is verbose as levels multiply.
- **Pratt / precedence climbing** — a single `parse_expr(min_bp)` driven by a
  binding-power table.

**Decision.** Use **Pratt parsing** for unary/binary expressions; the rest of the
grammar (items, statements) stays plain recursive descent.

**Rationale.** With Pratt, precedence and associativity live in one small
binding-power table, so adding an operator is a data change rather than a new
function — the design that scales as the operator set grows. It is a contained,
well-understood technique, not a speculative abstraction.

**Consequences.** Unary prefix binding power sits above all binary operators, so
`-a * b` parses as `(-a) * b`. New operators are added by extending the
binding-power table and the prefix/infix dispatch.

---

## ADR-0013 — AIR ratified design

**Status:** Accepted *(supersedes [ADR-0006](#adr-0006--air-is-a-typed-ssa-based-textual-verifiable-ir))*

**Context.** ADR-0006 recorded AIR's *direction* (typed, SSA, CFG, textual,
verifiable) but deferred the concrete design to its own milestone. M4 implements a
minimal AIR and lowering, which requires ratifying the representation.

**Decision.** AIR is a **typed, SSA-based IR over a CFG of basic blocks**, with an
**id/arena representation**:

- A `Function` owns flat arenas: instructions in a `Vec` addressed by `Value`, and
  basic blocks in a `Vec` addressed by `Block`. A block is an ordered list of the
  `Value`s it computes plus a `Terminator`.
- **Every value is the result of an instruction**, constants included (`iconst`),
  so operands are uniformly other `Value`s — no separate constant-operand concept.
- Instructions carry a result **type** and a source **span** (for later
  middle-end diagnostics and debug info).
- AIR is **frontend-independent**: `aether-air` depends only on `aether-source`.
  AST → AIR lowering is the separate `aether-lower` crate, and AIR defines its own
  operator enums (lowering maps `ast` ops onto them).

**Alternatives considered.**
- **LLVM-style "instruction *is* the value"** vs **Cranelift-style separate
  `Value`s with block parameters.** We took the middle path: instruction-as-value
  (1:1) for simplicity now. Whether control-flow SSA uses **phi nodes** or
  **block parameters** is deliberately left open until control flow lands (M6),
  since neither is needed for straight-line code.
- **A generic arena/interner crate (`aether-support`)** — not introduced; AIR's
  three `Vec` arenas are simple and specific, so a generic abstraction would be
  premature (TD-0010).

**Rationale.** The id/arena model is the counterpart to the AST's `Box` tree
(ADR-0011): it gives stable ids, cheap side tables, and cross-block references
without threading lifetimes — exactly what analyses, optimizations, and in-place
transformation need. Making constants instructions keeps the operand model
uniform. Keeping AIR frontend-independent preserves the layering in
`ARCHITECTURE.md`.

**Consequences.** The current implementation is intentionally minimal: a single
block per function, the instruction set needed for integer arithmetic
(`iconst`, `add`/`sub`/`mul`/`div`, `neg`) and a `ret` terminator, and one type
(`int`). A textual *parser* (full round-tripping), multi-block CFGs with
phi/block-parameters, dominance-based verification, and richer types/instructions
are future work tracked in `TECH_DEBT.md` and the roadmap.

---

## ADR-0014 — A program's result is printed to stdout

**Status:** Accepted

**Context.** With the interpreter (M5), `aetherc` can execute a program. The result
of `main` needs to be surfaced to the user, and running needs a place in the CLI.

**Alternatives considered.**
- **Use `main`'s return value as the process exit code** (Unix-style). Rejected as
  the default: exit codes are limited to 0–255 and would **collide** with the
  driver's own codes (a program returning `1` would be indistinguishable from a
  compile error, `2` from a usage error, etc.).
- **Gate execution behind an explicit `--run` flag.** Deferred: with no native
  codegen yet, running is the only useful thing to do with a valid program, so it
  is the natural default.

**Decision.** Executing is the **default** action of `aetherc <file>`; the
interpreter runs `main` and its integer result is **printed to stdout** (one
line). Successful execution exits `0`. The `--dump-*` flags remain explicit early
stops.

**Rationale.** Printing the value is unambiguous, unbounded, and easy to test,
and it keeps the driver's exit-code scheme intact. A distinct `RUNTIME_ERROR`
(exit 70) is used for runtime failures such as division by zero.

**Consequences.** The old "unimplemented beyond AIR" path (exit 3) is removed.
When native codegen arrives (Phase 4), the default may become "emit a binary"
with an explicit `--run`/`-i` for interpretation; this ADR would be revisited.

---

## ADR-0015 — Interpreter arithmetic: wrapping, with division-by-zero as a runtime error

**Status:** Accepted *(provisional; to be formalized with the type system, M8)*

**Context.** The interpreter must define the semantics of integer arithmetic,
including overflow and division by zero.

**Decision.** Integer arithmetic **wraps** (two's complement, via `wrapping_*`).
**Division by zero is a runtime error** (`RunError::DivisionByZero`) carrying the
offending source span, rendered as a caret diagnostic and exiting with
`RUNTIME_ERROR`.

**Rationale.** Wrapping is the simplest fully-defined behavior and also tames the
edge cases that would otherwise panic (`i64::MIN / -1`, `neg(i64::MIN)`). Division
by zero has no defined value, so it must be an error rather than a wrapped result.

**Consequences.** These are **provisional interpreter semantics**. The language's
real overflow policy (wrapping vs. checked vs. saturating) is a type-system-level
decision to be made in M8 (TD-0025); this ADR will be revisited then. There is no
runtime value type yet — results are `i64` — pending more types (TD-0024).

---

## ADR-0016 — Local variables lower via an SSA name environment; lowering resolves names (provisionally)

**Status:** Accepted *(provisional; name resolution moves to a dedicated pass at M9)*

**Context.** Local variables (`let x = …;` and references to `x`) need a
representation. AIR is SSA, and the M6 language is straight-line (no control flow
yet), so there are no control-flow merges.

**Decision.** Lower a local variable to the **SSA value of its initializer**, held
in a name → `Value` environment threaded through lowering. A `let` binds the name;
a name reference resolves to the bound value. Since resolution can fail (unknown
name), **lowering is fallible**: it returns a `LowerResult { module, diagnostics }`
and `aether-lower` depends on `aether-diagnostics`. An unknown name lowers to a
poison `iconst 0` plus an error diagnostic, keeping lowering total.

**Rationale.** With SSA and no control flow, a variable *is* just a named value —
no new AIR instructions, no runtime concept, and no interpreter changes are
needed. Threading an environment is the minimal mechanism. Lowering the
initializer before binding the name gives correct use-before-definition behavior
for free; a later `let` of the same name simply rebinds.

**Alternatives considered.**
- **A dedicated name-resolution pass** before lowering (producing a symbol table
  or resolved AST). This is the right long-term design and arrives at M9; doing it
  now would be premature infrastructure for one flat scope.
- **Interning names / a `Session`** — not needed; a `HashMap<String, Value>` is
  fine at this scale (TD-0010 holds).

**Consequences.** Name resolution currently lives in lowering as a stopgap
(TD-0026); the environment is a single flat scope with no nested-scope or formal
shadowing rules yet (TD-0027) — those arrive with control-flow blocks. `let` has
no type annotation (only `int` is inferred; the `:` token and annotations are
deferred, TD-0028). When the M9 name-resolution pass lands, lowering will assume
resolved names and this ADR will be revisited.

---

## ADR-0017 — SSA control-flow merges use block parameters, not phi nodes

**Status:** Accepted *(settles the question left open in
[ADR-0013](#adr-0013--air-ratified-design); implemented in M6 slice 2b)*

**Context.** Control flow (M6 slice 2) introduces basic blocks that merge: a
value's definition can depend on which predecessor edge executed (e.g.
`let m = if c { 10 } else { 20 };`). SSA needs a single name for such a merged
value. ADR-0013 deliberately deferred the choice between the two standard
representations until control flow landed; it must be made now because it shapes
the terminator set, the block structure, the verifier's dominance rule, the
interpreter's transfer logic, and every future analysis (TD-0019).

**Alternatives considered.**
- **Phi nodes** (LLVM, GCC, classic SSA). A `phi` pseudo-instruction at the top
  of a merge block selects a value per incoming predecessor:
  `%m = phi [%a, then], [%b, else]`. *Pros:* fits the current model with zero
  change (a phi is just another instruction, so `Value` stays "instruction
  result"); best-documented in the literature. *Cons:* a phi's operand list must
  stay in lockstep with the block's predecessor list, so every CFG edit (edge
  split, predecessor removal, jump threading) must rewrite phis — a well-known
  bug source; critical edges cannot carry the transfer and force edge-splitting;
  and the dominance invariant needs a special carve-out (a phi operand is not
  dominated by the phi's block but must dominate the end of its corresponding
  predecessor).
- **Block parameters** (Cranelift, MLIR, Swift SIL). Blocks take typed
  parameters like functions; each branch passes arguments along its edge:
  `br join(%a)` / `br join(%b)`, with `join(%m: int):`. *Pros:* the branch *is*
  the predecessor correspondence, so there is no separate list to keep in sync
  and CFG edits stay local; critical edges are handled naturally; one uniform
  dominance rule ("every use, including a branch argument, is dominated by its
  definition") with no phi carve-out. *Cons:* `Value` can no longer mean "result
  of an instruction" — it becomes "instruction result *or* block parameter",
  requiring a unified value table (a contained refactor of `aether-air`),
  superseding the 1:1 value↔instruction assumption of ADR-0013.

**Decision.** AIR represents SSA merges with **block parameters**. Branch
terminators carry per-edge argument lists; a block declares typed parameters that
its predecessors supply.

**Rationale.** This platform's charter prioritizes long-term architecture and
maintainability, and its roadmap is heavy on middle-end analyses/optimizations
(Phase 3) and codegen (Phase 4), where edge-splitting and CFG rewriting are
routine. Block parameters make those transformations local and keep a single,
uniform dominance invariant — the properties Cranelift and MLIR were designed
around, and the idiomatic choice for a Rust compiler. The one real cost, the
`Value`-model refactor, is best paid now while the IR is tiny rather than after
many passes assume value-is-instruction.

**Consequences.** Implemented in M6 slice 2b. This ADR is recorded up front (in
slice 2a) so the surrounding work aims at a consistent target — mirroring how
ADR-0006 recorded AIR's direction ahead of ADR-0013. Planned representation: a
unified `values` table where each value's definition is either an instruction
result or the *i*-th parameter of a block, each carrying its `Type`; `Terminator`
gains `br`/`condbr` variants carrying argument lists; the verifier moves to a
dominance-based def-before-use check that treats block parameters uniformly.
Straight-line code (through slice 2a) is unaffected: with no merges, no block
parameters are created and the current value model still holds. This settles the
open question in TD-0019.

---

## ADR-0018 — Booleans, comparisons, and a runtime value enum (provisional)

**Status:** Accepted *(provisional; formalized with the type system, M8)*

**Context.** M6 slice 2a adds booleans and comparison/logical operators as the
prerequisite for control flow, before a real type system exists (M8). Two things
need deciding provisionally: how `bool` and `int` coexist in AIR without a
checker, and how the interpreter represents values now that there is more than
one type.

**Decision.**
- AIR gains a second type, `Type::Bool`, alongside `Type::Int`, plus a boolean
  constant (`bconst`), an integer comparison instruction (`icmp <cond>`)
  producing `bool`, and a logical-not unary (`not`). Comparisons are their own
  instruction family (distinct from arithmetic `Binary`) because they map `int`s
  to a `bool`.
- Type consistency is enforced by the **AIR verifier** (there is no separate
  checker yet): `neg` and arithmetic require `int`, `not` requires `bool`,
  relational comparisons require `int`, equality (`==`/`!=`) requires both
  operands to share a type, and each instruction's declared result type must
  match what its operation produces. This is the provisional stand-in for
  semantic type checking (TD-0026, M8).
- The interpreter represents a runtime value as a public
  `RunValue { Int(i64), Bool(bool) }` enum. `interpret`/`run_function` return
  `RunValue`; the driver prints an `int` result as a number and a `bool` result
  as `true`/`false`.

**Rationale.** Landing the second type, its instructions, and the runtime value
enum in a straight-line slice de-risks control flow (slice 2b), which needs a
`bool` condition value and multi-typed runtime values regardless. Enforcing types
in the verifier keeps invalid AIR from reaching the interpreter without
prematurely building the M8 type system.

**Consequences.** Comparison operators parse left-associatively, so a chained
`a < b < c` is accepted syntactically and rejected only later by the verifier's
type check rather than by a friendly non-associativity error (TD-0029); a nicer
diagnostic awaits the type system. Unknown type *names* still fall back to `int`
(TD-0021). Short-circuiting `&&`/`||` are intentionally deferred to slice 2c
because their semantics require control flow. This introduces the runtime value
enum called for by TD-0024, which is now resolved. These interpreter/type
decisions are provisional and will be revisited when the type system (M8) and its
overflow policy (ADR-0015/TD-0025) are formalized.

---

## ADR-0019 — Control flow: statement-form `if`/`else`, CFG lowering, dominance by availability

**Status:** Accepted *(provisional; if-expressions and block-parameter merges arrive in slice 2c)*

**Context.** M6 slice 2b adds the first control flow. Several sub-decisions arise:
whether `if` is a statement or an expression, how lowering builds the CFG, how
block scoping works, and how the verifier checks SSA validity once a function has
more than one block.

**Decisions.**
- **`if`/`else` is a statement** (it produces no value). Both arms are blocks; an
  `else` may be a block or a chained `if` (`else if`). *Rationale:* with immutable
  `let` bindings and no assignment, a statement `if` never yields a value that must
  merge at the join, so it needs neither block parameters nor the value-model
  refactor (ADR-0017). This delivers real control flow in a focused, green slice.
  The **expression** form — which *does* force a merge — is deferred to slice 2c
  together with block parameters and short-circuit `&&`/`||`.
- **CFG lowering.** Lowering tracks a "current block" and, per statement sequence,
  whether control falls through or diverges (via `return`). An `if` splits into
  `then`/`else`/join blocks wired by `condbr`/`br`. A join block is created only
  when at least one arm falls through, so a both-arms-`return` `if` leaves no
  unreachable block.
- **Scoped environments.** The flat name map (TD-0027) becomes a stack of scopes;
  each braced block pushes a scope, so branch-local bindings are invisible after
  the `if` and shadowing is lexical.
- **Dominance by availability.** The verifier checks "a definition dominates its
  use" via a forward availability dataflow — the intersection, over a block's
  predecessors, of the values each makes available — replacing the old
  single-block index-order rule. Only reachable blocks are verified. This is
  correct for arbitrary CFGs; a dedicated dominator-tree analysis (M10) will later
  supersede the inline computation (TD-0030).

**Alternatives considered.** *Expression-form `if` now* — rejected for this slice:
it forces the block-parameter merge machinery, making the slice much larger; it is
scheduled next (2c). *Phi/dominator-tree in the verifier* — the availability
dataflow is simpler to implement correctly now and doubles as a gentle precursor
to the M10 dataflow engine; a real dominator tree is deferred to the analysis
framework.

**Consequences.** Statement `if`/`else`, `else if`, and nesting all parse, lower,
verify, and execute (the interpreter now follows the CFG). Resolves TD-0023
(entry-only interpreter) and TD-0027 (flat scope). TD-0019 shrinks to just
block-parameter merges (2c). If-expressions and `&&`/`||` remain deferred
(TD-0029). A function that does not `return` on all paths still surfaces as a
verifier "missing terminator" (TD-0020) until semantic analysis (M8).

---

## ADR-0020 — Block parameters implemented; short-circuit `&&`/`||`

**Status:** Accepted *(implements [ADR-0017](#adr-0017--ssa-control-flow-merges-use-block-parameters-not-phi-nodes))*

**Context.** ADR-0017 chose **block parameters** over phi nodes for SSA merges but
deferred the implementation. M6 slice 2c implements them, exercised by
short-circuiting `&&`/`||` — the minimal source construct that yields a value
merged from two paths.

**Decision.**
- **Unified value table.** `aether-air` moves from "`Value` = index into an
  instruction arena" to a table where each `Value` is a `ValueData { ty, span, def }`
  and `def` is either `ValueDef::Inst(InstData)` (an instruction result) or
  `ValueDef::Param { block, index }` (a block parameter). Because each instruction
  defines exactly one value, the instruction's data is stored **inline** in the
  `Inst` variant rather than in a separate arena — simpler than Cranelift's split,
  which exists to support multi-result instructions AIR does not have. This
  supersedes the 1:1 value↔instruction assumption noted in ADR-0013.
- **Branches carry arguments.** `Terminator::Br`/`CondBr` hold a
  `BranchTarget { block, args }` per edge; a predecessor supplies one argument per
  target parameter. `Terminator` is consequently no longer `Copy`.
- **Verifier.** A block's parameters are definitions at that block (available to it
  and the blocks it dominates); each branch's argument count and types must match
  the target's parameters. The dominance-by-availability check treats parameters
  uniformly — the single rule with no phi carve-out that ADR-0017 promised.
- **Interpreter.** On entering a block, its parameters are bound from the taken
  edge's arguments before the body runs.
- **`&&`/`||`.** Lowered to short-circuiting control flow: the right operand is
  evaluated only when it can change the result, and the paths merge at a block
  whose boolean parameter is the operator's value. No interpreter-specific
  short-circuit logic is needed — the CFG expresses it.

**Consequences.** SSA merges work end to end (TD-0019 resolved). `&&`/`||` combine
conditions and correctly skip a side-effecting right operand — e.g.
`false && (10 / 0 == 0)` returns `false` with no division-by-zero error (tested).
The value table stores instruction data inline, keeping the common accessors
(`value_type`, `value_def`, `value_span`) cheap. The remaining control-flow
feature, **if-expressions**, is still deferred (it needs block-with-tail-expression
language design; TD-0029). Overflow and type-checking provisions (ADR-0015/0018)
are unchanged.
