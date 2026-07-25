# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 2 — Language & Frontend Depth
**Current milestone:** M6 — Language expansion 🚧 (slices 1, 2a, 2b done)
**Next milestone:** M6 slice 2c — SSA merges (if-expressions & `&&`/`||`)

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Current milestone: M6 — Language expansion

M6 grows the language beyond a single `return`, in slices:

1. **Local variables & bindings** — ✅ **complete**.
2. **Control flow** — in three steps:
   - **2a — booleans & comparisons** — ✅ **complete**.
   - **2b — statement `if`/`else` & the CFG** — ✅ **complete** (this session).
   - **2c — SSA merges** (if-expressions, `&&`/`||`) — ⬜ next.
3. **Functions: parameters & calls** — ⬜.

### M6 slice 1 — local variables & bindings (complete)

- `aether-lexer`: added the `let` keyword and the `=` token.
- `aether-ast`: added `Stmt::Let` (a `LetStmt`) and `Expr::Name` (identifier
  reference); pretty-printer updated.
- `aether-parser`: parses `let NAME = <expr>;` and identifier expressions;
  the block loop now lowers multiple statements in order.
- `aether-lower`: threads a name → `Value` environment. A `let` binds a name to
  the SSA value of its initializer; a reference resolves to it. Because this
  resolves names, lowering is now fallible and returns `LowerResult { module,
  diagnostics }` (unknown names → a "cannot find name" diagnostic). No new AIR,
  runtime, or interpreter concepts were needed — a variable is just a named SSA
  value (ADR-0016).
- `aetherc`: consumes `LowerResult` and reports name-resolution errors.
- Tests: **119 total, all passing** (+1 lexer, +3 parser, +2 lower, +1 interp,
  +2 driver, net of doctest updates).

### M6 slice 2a — booleans & comparisons (complete)

The prerequisite for control flow, kept straight-line (no CFG yet). The pivotal
architectural decision this slice existed to make — **how AIR represents SSA
merges** — was settled up front: **block parameters, not phi nodes** (ADR-0017,
settling TD-0019); it is implemented next, in slice 2b.

- `aether-lexer`: comparison operators (`==`, `!=`, `<`, `<=`, `>`, `>=`), logical
  `!`, and the `true`/`false` keywords. Two-character operators (`==`, `!=`, `<=`,
  `>=`) share the one/two-char lookahead used for `->`.
- `aether-ast`: comparison variants on `BinOp` (with `is_comparison`), `UnOp::Not`,
  and `Expr::BoolLit`; pretty-printer updated.
- `aether-parser`: comparison precedence levels (equality < relational < additive
  < multiplicative < unary), `true`/`false` literals, and `!` as a prefix operator.
  Comparisons are left-associative for now (TD-0029).
- `aether-air`: a second type `Type::Bool`; `bconst`, `icmp <cond>` (a new `CmpOp`),
  and a `not` unary. The verifier became **type-aware** — it checks per-instruction
  operand/result types across `int`/`bool` (e.g. relational operands must be `int`,
  equality operands must match, `not` needs a `bool`).
- `aether-lower`: lowers bool literals, comparisons (to `icmp`), and `!`; `lower_type`
  now maps `int`→`Int` and `bool`→`Bool` (other names still fall back to `int`,
  TD-0021).
- `aether-air-interp`: a public runtime value enum `RunValue { Int, Bool }`
  (resolving TD-0024); `interpret`/`run_function` return it, and the driver prints
  `true`/`false` for a `bool` result. Arithmetic still wraps; div-by-zero unchanged.
- Decisions: ADR-0017 (block-parameter merges) and ADR-0018 (bool/comparison
  design + runtime value enum, provisional).
- Tests: **143 total, all passing** (+7 lexer, +5 parser, +2 ast, +5 air, +3 lower,
  +5 interp, +1 driver, net of doctest updates).

### M6 slice 2b — statement `if`/`else` & the CFG (complete)

The first real control-flow graph. `if`/`else` is a **statement** (no value yet);
with immutable `let`, nothing branch-computed is live at a join, so no SSA merges —
and therefore no block-parameter machinery — are needed. The AIR value model is
untouched (ADR-0019).

- `aether-lexer`: `if`/`else` keywords.
- `aether-ast`: `Stmt::If` (`IfStmt` + `ElseBranch`, supporting `else if` chains);
  pretty-printer prints `If`/`Then`/`Else`.
- `aether-parser`: `if <cond> { … } [else { … } | else if …]`. The condition is an
  expression; since expressions never begin with `{`, the block brace is
  unambiguous.
- `aether-air`: `Terminator::Br` and `Terminator::CondBr`, with a `successors()`
  helper; the printer emits `br blockN` / `condbr %c, blockT, blockE`.
- `aether-air` verifier: rewritten to be **CFG-aware** — reachability from entry,
  a forward *availability* dataflow that enforces "a definition dominates its use"
  across blocks, branch-target validity, and a `bool` `condbr` condition.
- `aether-lower`: builds the CFG (then/else/join blocks; a join is created only
  when an arm falls through, so both-arms-`return` leaves no dead block), and
  replaces the flat name map with a **scope stack** for lexical block scoping and
  shadowing (resolving TD-0027).
- `aether-air-interp`: `run_function` now **walks the CFG**, following
  `ret`/`br`/`condbr` between blocks (resolving TD-0023).
- Decision: ADR-0019 (statement-form `if`, CFG lowering, dominance by
  availability).
- Tests: **166 total, all passing** (+2 lexer, +4 parser, +2 air-print, +5
  air-verify, +5 lower, +6 interp, +1 driver, net of updates).

---

## Completed milestones

- **M5 — AIR interpreter** ✅ — `aether-air-interp`; `aetherc` runs programs and
  prints `main`'s result. (Phase 1 — First Light complete.)
- **M4 — AIR core & lowering** ✅ — `aether-air` + `aether-lower`; ADR-0013.
- **M3 — AST & parser** ✅ — `aether-ast` + `aether-parser` (Pratt).
- **M2 — Lexer** ✅ — `aether-lexer` (payload-free tokens).
- **M1 — Source & diagnostics** ✅ — `aether-source` + `aether-diagnostics`.
- **M0 — Project foundation** ✅ — workspace, tooling, CI, docs.

---

## Current progress

M6 slice 2b is finished; the workspace builds, lints cleanly, and all tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 166 passed, 0 failed

End-to-end: `aetherc file.ae` for
`fn main() -> int { let x = 10; let y = x - 3; return x * y; }` prints `70`;
`fn main() -> int { let n = 7; if n < 0 { return 1; } else if n == 0 { return 2; } else { return n * 2; } }`
prints `14`; an unknown variable prints "cannot find `…` in this scope" and
exits `1`.

---

## Next recommended milestone

**M6 slice 2c — SSA merges (if-expressions & `&&`/`||`).** This implements the
block parameters decided in ADR-0017 — the one piece of control flow slice 2b
deliberately deferred. It is where a value can depend on which branch executed.

Scope:
- **`aether-air` — the `Value`-model refactor** (do this first, in isolation).
  Move to a unified value table where a value's definition is either an instruction
  result or the *i*-th parameter of a block, each carrying its `Type`. Give
  `Br`/`CondBr` per-edge argument lists, and `Function` an API to append blocks
  with typed parameters. Update the printer (`br join(%a)`, `join(%p: int):`).
  This touches `ir.rs`, `print.rs`, `verify.rs`, `lower.rs`, and the interpreter,
  so land it as its own change with hand-built IR tests before adding syntax.
- **Verifier.** Extend the availability/dominance check to treat block parameters
  as definitions at their block, and validate branch argument arity and types
  against the target block's parameters.
- **Lexer/AST/parser.** `&&`/`||` operators (below equality in precedence); the
  **expression** form of `if` (`let m = if c { 10 } else { 20 };`) — likely an
  `Expr::If` with block bodies that yield a trailing value.
- **`aether-lower`.** Lower an if-expression by passing each arm's result to the
  join block as a branch argument; the join's parameter is the expression's value.
  Lower `&&`/`||` to short-circuiting branches with a bool-typed merge.
- **`aether-air-interp`.** Bind a block's parameters from the taken edge's
  arguments before executing the block body.
- Tests at every layer, plus end-to-end programs whose output is a value merged
  from both arms.

After 2c, control flow (slice 2) is complete and the next step is **slice 3 —
functions: parameters & calls**. Follow the standard workflow: plan, implement,
leave the repository green with updated docs.

---

## Architecture health

**Green.** Nine crates (eight libraries plus the `aetherc` binary), clean
one-directional dependencies. `aether-lower` also depends on `aether-diagnostics`
(a foundational crate — allowed by the dependency rules) because it performs
provisional name resolution. No cycles, no placeholder crates. AIR now has
multi-block functions with `br`/`condbr`, two types (`int`, `bool`), and a
CFG-aware verifier (reachability + dominance-by-availability + branch/type checks)
that stands in for real type checking until M8. Lowering uses a scope stack for
lexical block scoping. The one deliberate smudge — resolution living in lowering
(ADR-0016) — is tracked (TD-0026) with a clear exit (a dedicated pass at M9).
`aether-support` and a `Session` type remain unneeded (TD-0010). The one remaining
piece of control flow is SSA merges (block parameters, ADR-0017); implementing it
(slice 2c) entails a contained `Value`-model refactor.

---

## Outstanding work / technical debt

Nothing blocking. Tracked in [`TECH_DEBT.md`](TECH_DEBT.md). Resolved this slice:
entry-block-only interpreter (TD-0023) and single flat variable scope (TD-0027).
The remaining carry-over that slice 2c will resolve: SSA merges / block parameters
(TD-0019, decided per ADR-0017), which also unblocks if-expressions and `&&`/`||`
(TD-0029). New: the verifier recomputes dominance inline until the M10 analysis
framework (TD-0030). Others: name resolution in lowering (TD-0026, M9),
missing-return / literal-range checks pending M8 (TD-0020/0021), no `let` type
annotations (TD-0028), no AIR text parser (TD-0022), overflow policy provisional
(TD-0025), parser recovery/depth/params (TD-0016…0018), lexer limits
(TD-0011…0015), `Span` packing (TD-0006), diagnostic polish (TD-0007/8/9),
deferred `Session` (TD-0010), and the hand-rolled CLI → `clap` migration
(TD-0001).
