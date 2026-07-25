# Aether — Project Status

**Snapshot date:** 2026-07-25
**Current phase:** Phase 2 — Language & Frontend Depth
**Current milestone:** M6 — Language expansion 🚧 (slice 1 of 3 complete)
**Next milestone:** M6 slice 2 — control flow (`if`/`else`)

This document is the first thing to read at the start of a session. It reflects
the repository's actual state, which always takes precedence over any external
memory or conversation history.

---

## Current milestone: M6 — Language expansion

M6 grows the language beyond a single `return`, in slices:

1. **Local variables & bindings** — ✅ **complete** (this session).
2. **Control flow** (`if`/`else`, comparison & boolean operators) — ⬜ next.
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

M6 slice 1 is finished; the workspace builds, lints cleanly, and all tests pass.
There is no in-progress work carried into the next session.

**Verification (as of this snapshot):**
- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `cargo test --all` — 119 passed, 0 failed

End-to-end: `aetherc file.ae` for
`fn main() -> int { let x = 10; let y = x - 3; return x * y; }` prints `70`;
an unknown variable prints "cannot find `…` in this scope" and exits `1`.

---

## Next recommended milestone

**M6 slice 2 — control flow (`if`/`else`).** This is the significant one: it
introduces the first real control-flow graph.

Why now: local variables proved the frontend can grow while AIR stayed
single-block. Control flow is what finally exercises AIR's CFG design and forces
the decisions deferred since M4.

Suggested scope:
- Lexer: comparison operators (`==`, `!=`, `<`, `<=`, `>`, `>=` — note `==`/`!=`
  need two-char lookahead like `->`) and boolean operators (`&&`, `||`, `!`), plus
  keywords `if`/`else` (and likely `true`/`false` with a `bool` type).
- A `bool` type in AIR and a comparison instruction producing a boolean; the type
  system is still informal, so decide how `bool` and `int` coexist provisionally.
- AST: `if <cond> { … } else { … }` (as a statement and/or expression), boolean
  literals, comparison/logical expressions.
- **AIR: multiple basic blocks and branch terminators** (`br`, `condbr`). This is
  where the **phi-node vs. block-parameter** decision for SSA merges must be made
  and recorded as an ADR (TD-0019). Extend the verifier to dominance-based
  def-before-use across blocks.
- **Lowering**: build a CFG (then/else/join blocks); introduce **scoped**
  environments (a scope stack) for block bodies (TD-0027); handle SSA merges for
  values assigned on both branches.
- **Interpreter**: follow branches between blocks (TD-0023); execute phi /
  block-parameter transfers.
- Tests at every layer, plus end-to-end programs whose output depends on a branch.

This slice is itself large; if needed, split it (e.g. comparisons + `bool` first,
then `if`/`else` + CFG). Decide the SSA-merge representation early — it shapes
lowering, the interpreter, and every future analysis.

Follow the standard workflow: review theory and alternatives, record decisions,
plan, then implement — and leave the repository green with updated docs.

---

## Architecture health

**Green.** Eight crates, clean one-directional dependencies. `aether-lower` now
also depends on `aether-diagnostics` (a foundational crate — allowed by the
dependency rules) because it performs provisional name resolution. No cycles, no
placeholder crates. The one deliberate smudge — resolution living in lowering
(ADR-0016) — is tracked (TD-0026) with a clear exit (a dedicated pass at M9).
`aether-support` and a `Session` type remain unneeded (TD-0010): a flat
`HashMap<String, Value>` sufficed for the variable environment.

---

## Outstanding work / technical debt

Nothing blocking. Tracked in [`TECH_DEBT.md`](TECH_DEBT.md). New from M6 slice 1:
name resolution in lowering (TD-0026), single flat variable scope (TD-0027),
no `let` type annotations (TD-0028). The big carry-overs that M6 slice 2 will
resolve: single-block AIR / no phi (TD-0019), entry-block-only interpreter
(TD-0023). Others: missing-return / literal-range checks pending M8
(TD-0020/0021), no AIR text parser (TD-0022), runtime-value/overflow provisional
(TD-0024/0025), parser recovery/depth/params (TD-0016…0018), lexer limits
(TD-0011…0015), `Span` packing (TD-0006), diagnostic polish (TD-0007/8/9),
deferred `Session` (TD-0010), and the hand-rolled CLI → `clap` migration
(TD-0001).
