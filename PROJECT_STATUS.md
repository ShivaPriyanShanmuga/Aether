# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 2 — Language & Frontend Depth
**Current milestone:** M6 — Language expansion 🚧 (slice 1 done; slice 2a done)
**Next milestone:** M6 slice 2b — `if`/`else` & the CFG

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Current milestone: M6 — Language expansion

M6 grows the language beyond a single `return`, in slices:

1. **Local variables & bindings** — ✅ **complete**.
2. **Control flow** — in two steps:
   - **2a — booleans & comparisons** — ✅ **complete** (this session).
   - **2b — `if`/`else` & the CFG** — ⬜ next.
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

M6 slice 2a is finished; the workspace builds, lints cleanly, and all tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 143 passed, 0 failed

End-to-end: `aetherc file.ae` for
`fn main() -> int { let x = 10; let y = x - 3; return x * y; }` prints `70`;
`fn main() -> bool { let ok = 3 < 5; return ok == true; }` prints `true`;
an unknown variable prints "cannot find `…` in this scope" and exits `1`.

---

## Next recommended milestone

**M6 slice 2b — `if`/`else` & the CFG.** This builds the first real control-flow
graph on the foundation slice 2a laid (the `bool` type, comparisons, a type-aware
verifier, and a runtime value enum). The hard architectural question is already
answered: **SSA merges use block parameters, not phi nodes** (ADR-0017).

Scope:
- **`aether-air` — the `Value`-model refactor** (do this first). Move to a unified
  value table where a value's definition is either an instruction result or the
  *i*-th parameter of a block, each carrying its `Type`. Add `br`/`condbr`
  terminators that carry per-edge argument lists, and `Function` support for
  appending blocks with typed parameters. Update the printer
  (`br join(%a)`, `join(%p: int):`).
- **Verifier → dominance-based.** Replace the single-block "operand index <
  user index" rule with a dominance check that treats block parameters uniformly
  ("every use, including a branch argument, is dominated by its definition"), plus
  successor/argument-arity/type agreement on branches. (This is the payoff of the
  block-parameter choice — one uniform rule.)
- **Lexer/AST/parser.** Keywords `if`/`else`; `Stmt`/`Expr` for `if <cond> { … }
  else { … }` (decide statement vs. expression form — an expression form is what
  forces a real merge). Short-circuiting `&&`/`||` (lowered to branches, TD-0029).
- **`aether-lower` — build the CFG.** Then/else/join blocks; introduce **scoped**
  environments (a scope stack) for block bodies (TD-0027); pass values live out of
  both branches as block-parameter arguments on each edge.
- **`aether-air-interp` — CFG execution** (TD-0023). Follow `br`/`condbr` between
  blocks with a current-block loop; bind a block's parameters from the taken
  edge's arguments before executing its body.
- Tests at every layer, plus end-to-end programs whose output depends on a branch
  and on a value merged from both arms.

Recommended ordering within the slice: land the AIR value-model refactor +
terminators + dominance verifier first (with hand-built IR tests), then the
frontend and lowering, then interpreter CFG execution. If it proves too large for
one session, a clean cut is CFG-with-both-arms-returning (no merges) first, then
merges. Follow the standard workflow: plan, implement, leave the repository green
with updated docs.

---

## Architecture health

**Green.** Nine crates (eight libraries plus the `aetherc` binary), clean
one-directional dependencies. `aether-lower` also depends on `aether-diagnostics`
(a foundational crate — allowed by the dependency rules) because it performs
provisional name resolution. No cycles, no placeholder crates. AIR now carries two
types (`int`, `bool`) and a type-aware verifier stands in for real type checking
until M8. The one deliberate smudge — resolution living in lowering (ADR-0016) — is
tracked (TD-0026) with a clear exit (a dedicated pass at M9). `aether-support` and
a `Session` type remain unneeded (TD-0010): a flat `HashMap<String, Value>` still
serves the variable environment. AIR's SSA-merge representation is now decided
(block parameters, ADR-0017); implementing it (slice 2b) entails a contained
`Value`-model refactor.

---

## Outstanding work / technical debt

Nothing blocking. Tracked in [`TECH_DEBT.md`](TECH_DEBT.md). Resolved this slice:
the runtime value enum (TD-0024). New from slice 2a: comparison associativity /
deferred `&&`/`||` (TD-0029). The big carry-overs that slice 2b will resolve:
single-block AIR / block-parameter merges (TD-0019, now *decided* per ADR-0017),
entry-block-only interpreter (TD-0023), and single flat variable scope (TD-0027).
Others: name resolution in lowering (TD-0026, M9), missing-return / literal-range
checks pending M8 (TD-0020/0021), no `let` type annotations (TD-0028), no AIR text
parser (TD-0022), overflow policy provisional (TD-0025), parser recovery/depth/
params (TD-0016…0018), lexer limits (TD-0011…0015), `Span` packing (TD-0006),
diagnostic polish (TD-0007/8/9), deferred `Session` (TD-0010), and the hand-rolled
CLI → `clap` migration (TD-0001).
