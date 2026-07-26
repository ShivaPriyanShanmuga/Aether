# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 2 — Language & Frontend Depth
**Current milestone:** M6 — Language expansion ✅ **complete** (this session: slice 3)
**Next milestone:** M7 — Name resolution & scopes

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Last milestone: M6 — Language expansion (complete)

M6 grew the language beyond a single `return`, in slices:

1. **Local variables & bindings** — ✅ **complete**.
2. **Control flow** — ✅ **complete** (statement forms), in three steps:
   - **2a — booleans & comparisons** — ✅ **complete**.
   - **2b — statement `if`/`else` & the CFG** — ✅ **complete**.
   - **2c — SSA merges & short-circuit `&&`/`||`** — ✅ **complete**.
     (If-*expressions* remain an optional deferred slice — TD-0029.)
3. **Functions: parameters & calls** — ✅ **complete** (this session).

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

### M6 slice 2c — SSA merges & short-circuit `&&`/`||` (complete)

Implements the block parameters chosen in ADR-0017 (now ADR-0020) — the one
control-flow piece deferred from 2b — exercised by short-circuit logical operators.

- `aether-air`: the **`Value`-model refactor**. A value is now an entry in a
  unified table (`ValueData { ty, span, def }`) whose `def` is either
  `ValueDef::Inst(InstData)` (an instruction result, data stored inline) or
  `ValueDef::Param { block, index }` (a block parameter). `Br`/`CondBr` carry a
  `BranchTarget { block, args }` per edge (so `Terminator` is no longer `Copy`).
  New API: `append_block_param`, `value_def`, `value_span`. The printer emits
  `blockN(%p: ty):` headers and `br blockN(%a)` arguments.
- `aether-air` verifier: block parameters are definitions at their block; each
  branch's argument count and types must match the target's parameters (the
  dominance-by-availability check now treats parameters uniformly).
- `aether-air-interp`: binds a block's parameters from the taken edge's arguments
  before running the block; `eval` reads `value_def`.
- `aether-lexer`/`aether-ast`/`aether-parser`: `&&`/`||` tokens and `BinOp`
  variants; precedence `|| < && < equality < relational < additive <
  multiplicative` (a lone `&`/`|` is a lexical error).
- `aether-lower`: `lower_expr` now threads `self.current` (expressions can branch);
  `&&`/`||` lower to a short-circuit diamond that merges via a `bool` block
  parameter. The right operand is skipped when it cannot change the result — so
  `false && (10 / 0 == 0)` yields `false` with no runtime error.
- Decision: ADR-0020 (block parameters implemented; short-circuit `&&`/`||`).
  Resolves TD-0019.
- Tests: **182 total, all passing** (+2 lexer, +2 parser, +1 ast, +4 air, +5
  interp, +2 lower, +1 driver, net of updates).

### M6 slice 3 — functions: parameters & calls (complete)

User-defined functions that take arguments and call one another, including
recursion. **Function parameters are entry-block parameters** (ADR-0021), reusing
the block-parameter machinery from 2c: a `call` binds the callee's entry
parameters from its arguments exactly as a branch binds a block's.

- `aether-lexer`: a `:` token.
- `aether-ast`: `Param` and `FnDecl.params`; `Expr::Call { callee, args }`; the
  pretty-printer prints `Param`/`Call` nodes.
- `aether-parser`: `fn NAME(name: type, …) -> TYPE`; call expressions
  `callee(args)` (an `Ident` followed by `(`).
- `aether-air`: `InstData::Call { callee, args }` (referenced by name, ADR-0021),
  which makes `InstData` `Clone` rather than `Copy`; `Function::append_param`/
  `params()`; `Module::function_by_name`. The printer shows the signature
  `fn add(%0: int, %1: int)` and `call add(%0, %1)`.
- `aether-air` verifier: threads the `Module`; a call's argument arity and types
  must match the callee's signature, and the result type is the callee's return
  type; an unknown callee is caught.
- `aether-lower`: a signature pre-pass (name → return type) types call results;
  parameters lower to entry-block parameters bound by name; calls lower to the
  `call` instruction; an unknown callee is a "cannot find function" diagnostic.
- `aether-air-interp`: threads the `Module`; a `call` runs the callee in a fresh
  frame with arguments bound to its entry parameters (recursion via the host
  stack, TD-0031); runtime errors propagate out of calls.
- Decision: ADR-0021. Resolves TD-0018.
- Tests: **202 total, all passing** (+2 lexer, +5 parser, +3 lower, +6 interp,
  +2 air, +2 driver, net of updates).

---

## Completed milestones

- **M6 — Language expansion** ✅ — locals, control flow (`if`/`else`, comparisons,
  booleans, short-circuit `&&`/`||`), and functions with parameters, calls, and
  recursion. Details above.
- **M5 — AIR interpreter** ✅ — `aether-air-interp`; `aetherc` runs programs and
  prints `main`'s result. (Phase 1 — First Light complete.)
- **M4 — AIR core & lowering** ✅ — `aether-air` + `aether-lower`; ADR-0013.
- **M3 — AST & parser** ✅ — `aether-ast` + `aether-parser` (Pratt).
- **M2 — Lexer** ✅ — `aether-lexer` (payload-free tokens).
- **M1 — Source & diagnostics** ✅ — `aether-source` + `aether-diagnostics`.
- **M0 — Project foundation** ✅ — workspace, tooling, CI, docs.

---

## Current progress

M6 (slice 3, the final slice) is finished; M6 is complete. The workspace builds,
lints cleanly, and all tests pass. There is no in-progress work carried into the
next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 202 passed, 0 failed

End-to-end: `aetherc file.ae` for
`fn main() -> int { let x = 10; let y = x - 3; return x * y; }` prints `70`;
`fn main() -> bool { let x = 5; return x > 0 && x < 10; }` prints `true`; and the
recursive
`fn fact(n: int) -> int { if n <= 1 { return 1; } return n * fact(n - 1); } fn main() -> int { return fact(5); }`
prints `120`. An unknown variable/function prints "cannot find …" and exits `1`.

---

## Next recommended milestone

**M7 — Name resolution & scopes.** M6 is complete: the language now has locals,
control flow, and functions. The natural next step is to lift name resolution out
of lowering into a dedicated pass, which the codebase has been deferring since
ADR-0016.

Why now: lowering currently doubles as a resolver (local names *and* call targets),
which makes it fallible and mixes concerns (TD-0026). With functions in place there
are two kinds of names to resolve (locals and functions) and real scope/shadowing
rules to formalize, so a dedicated pass earns its place.

Suggested scope:
- A new `aether-sema` crate (or module) housing a **name-resolution pass** that
  walks the AST, builds scopes, and resolves each identifier to a binding
  (local/parameter) and each call to a target function — producing resolved
  references (a step toward the `FuncRef`/`DefId` deferred in ADR-0021).
- Formalize scope and shadowing rules; diagnose unknown names, unknown functions,
  and (now) **duplicate function names** (TD-0031) here rather than in lowering.
- Make lowering **infallible** again: it consumes resolved names and no longer
  emits diagnostics (reverting the ADR-0016 stopgap; TD-0026).
- Decide the resolved representation (resolved AST vs. a side table) — an ADR.

This sets up the **type system (M8)**, which builds on resolved names. An
alternative smaller piece is **if-expressions** (deferred M6 polish; needs
block-with-tail-expression design, TD-0029). Follow the standard workflow: present
theory and alternatives, record decisions, plan, then implement — leaving the
repository green with updated docs.

---

## Architecture health

**Green.** Nine crates (eight libraries plus the `aetherc` binary), clean
one-directional dependencies. `aether-lower` also depends on `aether-diagnostics`
(a foundational crate — allowed by the dependency rules) because it performs
provisional name resolution (local names and call targets). No cycles, no
placeholder crates. AIR is now a full SSA CFG: multi-block functions with
`br`/`condbr`, a **unified value table** (instruction results and block
parameters), SSA merges via block parameters (ADR-0017/0020), function parameters
as entry-block parameters with a `call` instruction (ADR-0021), two types (`int`,
`bool`), and a CFG-aware verifier (reachability + dominance-by-availability +
branch/call arg & type checks) standing in for real type checking until M8.
Lowering uses a scope stack for lexical block scoping and threads a current block
so expressions (`&&`/`||`) can branch. The one deliberate smudge — resolution
living in lowering (ADR-0016) — is tracked (TD-0026) with a clear exit (a dedicated
pass at M7). `aether-support` and a `Session` type remain unneeded (TD-0010).

---

## Outstanding work / technical debt

Nothing blocking. Tracked in [`TECH_DEBT.md`](TECH_DEBT.md). Resolved this slice:
function parameters (TD-0018). New: interpreter recursion on the host stack, and
undiagnosed duplicate function names (TD-0031). The next milestone (M7) will
resolve: name resolution in lowering (TD-0026). Others: if-expressions (TD-0029,
optional M6 polish), inline dominance in the verifier (TD-0030, share the M10
analysis), missing-return / literal-range checks pending M8 (TD-0020/0021), no
`let` type annotations (TD-0028), no AIR text parser (TD-0022), overflow policy
provisional (TD-0025), parser recovery/depth (TD-0016/0017), lexer limits
(TD-0011…0015), `Span` packing (TD-0006), diagnostic polish (TD-0007/8/9),
deferred `Session` (TD-0010), and the hand-rolled CLI → `clap` migration
(TD-0001).
