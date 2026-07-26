# Aether — Session Handoff

> A complete, self-contained context handoff. Paste this as the opening prompt for
> the next session. It reflects the repository as of commit **`b2120b0`**
> (M6 complete). The **repository is the source of truth** — if this document and
> the repo ever disagree, trust the repo and update this file.

---

## 0. Your role & the project

You are my senior compiler engineer, software architect, and long-term
collaborator on **Aether**, a production-quality compiler platform in Rust
(inspired by LLVM, Clang, rustc, GCC, MLIR, Cranelift). This is a
**portfolio-quality, long-lived** project: prioritize architecture, correctness,
maintainability, and documentation over speed. Do **not** build features
prematurely or add unjustified abstractions.

- **Repo:** `c:\Users\mahas\OneDrive\Desktop\to do in life\repos\Aether`
- **Git:** GitHub `origin` = `ShivaPriyanShanmuga/Aether`, branch `main`.
- **Latest commit:** `b2120b0` — "M6 (slice 3): functions with parameters, calls, and recursion".
- **Toolchain:** Rust **1.89.0** pinned (`rust-toolchain.toml`), edition **2024**,
  resolver **3**, MSRV 1.89. ~7,600 lines of Rust across 9 crates. **202 tests**,
  fmt/clippy clean.

---

## 1. FIRST — before doing anything else

The repository's own documents are the handoff. Read these, in order, to reload
full context (this file summarizes them but they are authoritative):

1. **PROJECT_STATUS.md** — current position + the detailed next-milestone plan.
2. **ROADMAP.md** — phases/milestones and their status.
3. **DECISIONS.md** — all ADRs (ADR-0001 … ADR-0021); the *why* behind everything.
4. **ARCHITECTURE.md** — pipeline, crate graph, dependency rules, AIR design.
5. **TECH_DEBT.md** — TD-0001 … TD-0031 (known limitations + triggers + severity).
6. **CONTRIBUTING.md** — coding standards and session workflow.

Then confirm a **green baseline** before coding (see §8 Green gates).

---

## 2. Where we are

- **Phase 0 (Foundation):** complete — M0.
- **Phase 1 (First Light):** complete — M1–M5. A program compiles and **runs** end
  to end: source → tokens → AST → AIR (our SSA IR) → interpreter → printed result.
- **Phase 2 (Language & Frontend Depth):** **M6 (Language expansion) is COMPLETE.**
  - slice 1 — local variables & bindings ✅
  - slice 2 — control flow ✅ (2a booleans/comparisons, 2b statement `if`/`else` +
    CFG, 2c SSA merges via block parameters + short-circuit `&&`/`||`)
  - slice 3 — functions: parameters, calls, recursion ✅
- **Next milestone: M7 — Name resolution & scopes** (see §9).

---

## 3. Milestone history (every session so far)

| Commit | Milestone | Delivered |
| --- | --- | --- |
| `69e7df9` | **M0** Project foundation | Cargo workspace, pinned toolchain, lint policy, CI, license, the seven project docs, `aetherc` driver skeleton. |
| `c7248e5` | **M1** Source & diagnostics | `aether-source` (`SourceMap`, `Span`, byte→line/col), `aether-diagnostics` (structured diagnostics + caret rendering). |
| `a3e61c7` | **M2** Lexer | `aether-lexer`: payload-free `Copy` tokens, error recovery; `--dump-tokens`. |
| `7c6519a` | **M3** AST & parser | `aether-ast` (`Box` tree + pretty-printer), `aether-parser` (recursive descent + Pratt); `--dump-ast`. |
| `f2d4bb1` | **M4** AIR core & lowering | `aether-air` (typed SSA id/arena IR + printer + verifier), `aether-lower`; ADR-0013; `--dump-air`. |
| `aee610b` | **M5** AIR interpreter | `aether-air-interp`; `aetherc` runs programs and prints `main`'s result. **First end-to-end pipeline.** |
| `55e4429` | **M6 slice 1** locals | `let`, `=`, identifier expressions, name→value env in lowering (ADR-0016). |
| `1a4b9a7` | docs | crate-count correction. |
| `f1f2cf7` | **M6 slice 2a** booleans/comparisons | `bool` type, `true`/`false`, `== != < <= > >=`, `!`, type-aware verifier, `RunValue` enum. ADR-0017 (decide block params), ADR-0018. |
| `d5ff0c4` | **M6 slice 2b** statement `if`/`else` + CFG | multi-block AIR, `br`/`condbr`, dominance-by-availability verifier, scoped envs, CFG interpreter. ADR-0019. |
| `b255550` | **M6 slice 2c** SSA merges + `&&`/`||` | unified value table + block parameters (implements ADR-0017), short-circuit logical ops. ADR-0020. |
| `b2120b0` | **M6 slice 3** functions | `:` token, params, calls, recursion; params are entry-block params. ADR-0021. |

---

## 4. What EXISTS — the crates (do NOT rebuild; extend these)

Nine crates in a Cargo workspace (edition 2024, resolver 3, pinned Rust 1.89.0).
Dependencies flow **frontend → IR → backend**, with foundational crates at the
bottom; the driver sits on top. No cycles.

- **`aether-source`** — `BytePos`, `FileId`, `Span` (private repr behind accessors,
  ADR-0008), `LineCol`, `SourceFile`, `SourceMap`; byte→line/column (UTF-8/CRLF
  aware). No project deps.
- **`aether-diagnostics`** — `Diagnostic` (severity, optional code, primary/secondary
  labeled spans, notes) via a fluent builder; `DiagnosticHandler`; `render()`
  producing rustc-style caret output (plain text). → source.
- **`aether-lexer`** — payload-free `Copy` `TokenKind` + `Token{kind,span}`;
  `tokenize(&SourceFile) → LexResult{tokens, diagnostics}`. Tokens: `Ident`, `Int`;
  keywords `fn return let true false if else`; delimiters `( ) { } ; : ,`;
  operators `+ - * / -> = == != < <= > >= ! && ||`; `Eof`. `//` line comments;
  two-char lookahead (`->`, `==`, `!=`, `<=`, `>=`, `&&`, `||`); error recovery.
  Lexemes recovered via span (ADR-0010, no interning). → source, diagnostics.
- **`aether-ast`** — `Box`-owned tree (ADR-0011): `Program`, `Item::Fn(FnDecl)`,
  `FnDecl{name, params, return_type, body, span}`, `Param{name, ty, span}`, `Type`,
  `Block`, `Stmt::{Let, Return, If}`, `IfStmt{cond, then_block, else_branch}`,
  `ElseBranch::{Block, If}`, `Expr::{IntLit, BoolLit, Unary, Binary, Name, Call,
  Error}`, `BinOp{Add Sub Mul Div Eq Ne Lt Le Gt Ge And Or}`, `UnOp{Neg, Not}`,
  `Ident`. Self-contained; `pretty::print` (source-map-free, for golden tests). → source.
- **`aether-parser`** — recursive descent + Pratt expressions (ADR-0012);
  `parse(&SourceFile, &[Token]) → ParseResult{program, diagnostics}`. Precedence
  (loosest→tightest): `|| && ==/!= </<=/>/>= +/- */÷` then unary prefix `- !`; calls
  are postfix on a bare identifier. Error recovery (`Expr::Error` poison nodes, sync
  to next fn). → ast, lexer, source, diagnostics.
- **`aether-air`** — AIR: typed, SSA, id/arena IR (ADR-0013/0017/0020). See §6. → source.
- **`aether-lower`** — `lower(&Program) → LowerResult{module, diagnostics}`.
  Post-order, naturally SSA. Threads a **scope stack** (`Vec<HashMap<String,Value>>`)
  for lexical block scoping (TD-0027 resolved) and a **current block** so
  expressions can branch. A **signature pre-pass** records each function's return
  type to type call results. Does **provisional** name + callee resolution
  (ADR-0016/0021), so it is fallible (unknown name/function → diagnostic). Lowers
  `&&`/`||` to short-circuit CFG diamonds merging via a bool block parameter.
  → ast, air, source, diagnostics.
- **`aether-air-interp`** — `interpret(&Module) → Result<RunValue, RunError>`;
  `run_function(&Module, &Function, &[RunValue])`. `RunValue{Int(i64), Bool(bool)}`
  (impl `Display`). `RunError::{NoEntryPoint, DivisionByZero{span}}`. Walks the CFG
  (follows `ret`/`br`/`condbr`, binds block/entry params from edge/call args);
  **recursive call frames**. Wrapping two's-complement arithmetic; div-by-zero is a
  runtime error carrying a span (ADR-0015). → air, source.
- **`aetherc`** (binary) — the driver. DEFAULT action runs the program and prints
  `main`'s result to stdout (ADR-0014). Flags: `--dump-tokens`, `--dump-ast`,
  `--dump-air` (each stops after its phase), `--help/-h`, `--version/-V`.
  Hand-rolled arg parsing (clap deferred, TD-0001). Exit codes: 0 success,
  1 compile error, 2 usage, 70 runtime error, 74 IO. → all phases.

---

## 5. The language accepted today

```
// one or more functions; only `main` is executed, and its result is printed.
fn NAME(p1: TYPE, p2: TYPE, ...) -> TYPE { STMT* }
```

- **Types:** `int` (64-bit signed) and `bool`. Unknown type names still lower to
  `int` provisionally (TD-0021).
- **Statements:** `let NAME = EXPR;` (lexically block-scoped, shadowing allowed);
  `return EXPR;`; `if COND { .. } else { .. }` / `else if ..` / no-`else`
  (statement form — no value yet; if-*expressions* deferred, TD-0029).
- **Expressions:** integer & boolean literals; identifiers (locals/params);
  arithmetic `+ - * /`; comparisons `== != < <= > >=`; unary `-` and `!`;
  short-circuit `&&`/`||`; parentheses; function calls `f(args)` (callee is a bare
  name; recursion works).
- **Semantics:** wrapping integer arithmetic; division by zero is a runtime error;
  `&&`/`||` short-circuit (a skipped operand's side effects, e.g. `1/0`, never run).
- **Source extension:** `.ae`.

Example that runs and prints `120`:
```
fn fact(n: int) -> int { if n <= 1 { return 1; } return n * fact(n - 1); }
fn main() -> int { return fact(5); }
```

---

## 6. AIR design (current state)

Typed, SSA-based IR over a CFG of basic blocks, id/arena representation
(ADR-0013), with block-parameter SSA merges (ADR-0017/0020).

- `Module` — a `Vec<Function>`; `functions()`, `function_by_name(&str)`.
- `Function` — `name`, `return_type`, a **unified value table** `Vec<ValueData>`
  (addressed by `Value`), a block arena `Vec<BlockData>` (addressed by `Block`),
  and `entry`. A **function's parameters are its entry block's parameters**
  (`append_param`, `params()`, ADR-0021). Accessors: `value_def`, `value_type`,
  `value_span`, `value_count`, `block`, `blocks`, `push_inst`, `append_block`,
  `append_block_param`, `set_terminator`.
- `ValueData{ def, ty, span }`; `ValueDef::Inst(InstData) | Param{block, index}`.
  (Instruction data is stored **inline** because each instruction defines exactly
  one value — simpler than Cranelift's split, ADR-0020.)
- `InstData` (**`Clone`, not `Copy`**): `IConst(i64)`, `BConst(bool)`,
  `Unary{op,operand}`, `Binary{op,lhs,rhs}` (int), `ICmp{op,lhs,rhs}` (→ bool),
  `Call{callee: String, args: Vec<Value>}`. Ops: `UnaryOp{Neg,Not}`,
  `BinaryOp{Add,Sub,Mul,Div}`, `CmpOp{Eq,Ne,Lt,Le,Gt,Ge}`.
- `BlockData{ params: Vec<Value>, body: Vec<Value>, terminator: Option<Terminator> }`.
- `Terminator` (`Clone`, not `Copy`): `Ret(Value)`, `Br(BranchTarget)`,
  `CondBr{cond, then_branch, else_branch}`. `BranchTarget{block, args: Vec<Value>}`
  (`new`, `with_args`); `successors()`. Each edge carries arguments for its target's
  parameters; a `call` carries arguments for the callee's entry parameters.
- **Textual form** — `print(&Module)` (used by `--dump-air` and golden tests):
  `fn f(%0: int) -> int {`, `blockN(%p: ty):` headers (entry params shown in the
  signature instead), `br blockN(%a)`, `condbr %c, blockT, blockE`, `call f(%a, %b)`.
  There is **no textual parser** yet (TD-0022).
- **Verifier** — `verify(&Module) → Vec<VerifyError>`. Checks: every reachable block
  terminated & branch targets exist; **definition dominates use** (computed as a
  forward *availability* dataflow — intersection over predecessors; block/entry
  params count as defs); per-instruction operand/result types (`int`/`bool`);
  `condbr` condition is `bool`; branch **and call** argument arity/types match the
  target's parameters/callee's signature; `ret` type matches the function's return
  type. Only reachable blocks are checked. Dominance is recomputed inline for now
  (TD-0030; a real dominator-tree analysis lands with M10).

---

## 7. Key decisions locked (ADRs — honor these; don't re-litigate)

Rust (0001); custom minimal `.ae` language (0002); thin vertical slice first
(0003); interpreter-first (0004); Cargo workspace per subsystem (0005); AIR
direction [superseded by 0013] (0006); dependency-minimal hand-rolled CLI (0007);
per-file byte-range `Span` with private repr (0008); diagnostics split
construction/collection/rendering (0009); payload-free tokens, interning deferred
(0010); AST = `Box` tree (0011); Pratt parser (0012); AIR = typed SSA id/arena,
frontend-independent (0013); print result to stdout, running is default (0014);
wrapping arithmetic + div-by-zero runtime error, provisional (0015); locals via SSA
name env, resolution in lowering provisionally (0016); **SSA merges use block
parameters, not phi nodes** (0017); booleans/comparisons + runtime value enum,
provisional (0018); statement-form `if`/`else`, CFG lowering, dominance by
availability (0019); block parameters implemented + short-circuit `&&`/`||` (0020);
functions — parameters as entry-block parameters, calls by name provisionally
(0021). ADRs are **append-only/immutable once Accepted**; supersede with a new one.

---

## 8. Working agreement (from CONTRIBUTING.md — follow every session)

- **Architecture BEFORE implementation.** For any non-trivial subsystem: present the
  theory, weigh 2–3 alternatives with tradeoffs, recommend one, record an **ADR**,
  plan, THEN code.
- **One milestone (or one slice) per session.** Leave the repo green and continuable.
- **Green gates** (all must pass before committing):
  ```
  cargo fmt --all -- --check
  cargo clippy --all-targets -- -D warnings
  cargo build --all-targets
  cargo test --all
  ```
  Workspace lints are strict: `unsafe_code=deny`, `missing_docs`,
  `missing_debug_implementations`, `unreachable_pub`, `clippy::all` (all `warn`, but
  CI/clippy runs `-D warnings`). Document all public items. Library code returns
  `Result` and avoids panics on reachable input (`expect`/`unreachable!` only on
  genuine invariants — e.g. "verified AIR" — with a message).
- Prefer golden/snapshot tests (AST pretty-print, AIR textual form) plus unit +
  integration tests. Add a test with every behavior/bug fix.
- At session end: update PROJECT_STATUS.md, ROADMAP.md, ARCHITECTURE.md,
  DECISIONS.md, TECH_DEBT.md as needed; record new ADRs and tech debt; recommend the
  single logical next milestone in PROJECT_STATUS.md. Then commit and push to
  `origin/main`.
- **IMPORTANT: do NOT add a `Co-Authored-By: Claude` trailer to commits.** Existing
  history has none; author/committer stays ShivaPriyanShanmuga. (Also in the memory
  file.)

---

## 9. The next task — M7: Name resolution & scopes

**Why now.** M6 is complete; the language has locals, control flow, and functions.
Lowering currently *doubles as a resolver* for both local names and call targets,
which makes it fallible and mixes concerns (the ADR-0016 stopgap; TD-0026). With
functions in place there are two kinds of names and real scope/shadowing rules, so
a dedicated pass now earns its place — and it sets up the type system (M8).

**Suggested scope (present a plan + record ADRs before coding):**
- A new **`aether-sema`** crate (or module) housing a **name-resolution pass** that
  walks the AST, builds scopes, and resolves each identifier to a binding
  (local/parameter) and each call to a target function — producing **resolved
  references** (the step toward the `FuncRef`/`DefId` deferred in ADR-0021).
- Decide the resolved representation (resolved AST vs. a side table) — **an ADR**.
- Formalize scope & shadowing rules; diagnose unknown names, unknown functions, and
  (now) **duplicate function names** (TD-0031) here rather than in lowering.
- Make **lowering infallible again**: it consumes resolved names and stops emitting
  diagnostics (reverts the ADR-0016 stopgap; pays down TD-0026).

**Alternative smaller slice:** **if-expressions** (deferred M6 polish; needs
block-with-tail-expression language design, but the block-parameter merge machinery
it needs already exists — TD-0029).

After M7 → **M8 (type system & checking)**, then Phase 3 (M9 pass/analysis
framework, M10 analyses, M11 optimizations), Phase 4 (native backend), etc. See
ROADMAP.md.

---

## 10. Tech debt snapshot (see TECH_DEBT.md for full detail + triggers)

**Resolved:** TD-0002 (pipeline stub, M5), TD-0018 (function params, M6.3),
TD-0019 (SSA merges/block params, M6.2c), TD-0023 (entry-only interpreter, M6.2b),
TD-0024 (i64-only runtime values, M6.2a), TD-0027 (flat variable scope, M6.2b).

**Open (highlights):** TD-0026 name resolution in lowering (→ M7); TD-0020
missing-return surfaces as a verify error & TD-0021 literal-range/type-name
unchecked & TD-0025 overflow policy & TD-0028 no `let` type annotations (→ M8);
TD-0029 left-associative comparisons + no if-expressions; TD-0030 verifier
recomputes dominance inline (→ share M10 analysis); TD-0031 interpreter recursion
on the host stack + undiagnosed duplicate function names; TD-0022 no AIR text
parser; TD-0016/0017 parser recovery/depth; TD-0011…0015 lexer limits; TD-0006
`Span` packing; TD-0007/8/9 diagnostic polish; TD-0010 deferred `Session`;
TD-0001 hand-rolled CLI → `clap`; TD-0003/0004/0005 licensing/CI.

---

## 11. Deliberately deferred (do NOT add prematurely)

- `aether-support` and a `Session`/context type — still unneeded after M6 (TD-0010).
- String/symbol interning (ADR-0010); a `FuncRef`/`DefId` abstraction (arrives with
  M7 name resolution); the type system (M8); an AIR textual parser (TD-0022); the
  `clap` CLI migration (TD-0001).

---

## 12. Note on a corrected cross-reference

ROADMAP.md places **name resolution at M7**. Some early text (immutable **ADR-0016**)
still says "M9"; M9 is actually the *pass/analysis framework* (Phase 3). The mutable
docs and newer ADRs (0021) were corrected to **M7**; ADR-0016 is left as a historical
record (ADRs are immutable). Treat **M7 = name resolution** as authoritative.
