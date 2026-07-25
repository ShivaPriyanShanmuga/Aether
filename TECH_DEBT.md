# Aether — Technical Debt

Known limitations, simplifications, and planned refactors. Technical debt is
never left undocumented: when a milestone takes a deliberate shortcut, it is
recorded here with enough context to pay it down later.

Each item notes its **impact**, the **trigger** that should prompt action, and a
**severity** (low / medium / high).

---

## Open items

### TD-0001 — Hand-rolled CLI argument parsing
- **Severity:** low
- **Context.** `aetherc` parses arguments by hand (see `crates/aetherc/src/cli.rs`)
  to keep the foundation dependency-free (ADR-0007). It supports only
  `--help`/`-h`, `--version`/`-V`, and a single input path.
- **Impact.** Fine today; will not scale to subcommands, grouped flags, or
  `--`-style option termination.
- **Trigger.** The first time we need subcommands (e.g. `aetherc build`,
  `aetherc emit-air`) or more than a handful of flags, migrate to `clap` (derive).
- **Notes.** `--` (end-of-options) is currently treated as an unknown option;
  fold this into the `clap` migration.

### TD-0002 — `aetherc` compilation pipeline is a stub — ✅ resolved (M5)
- **Severity:** low (by design)
- **Context.** The `compile` path originally read the input and reported the
  pipeline as unimplemented (exit code 3).
- **Resolution.** Paid down incrementally across Phase 1; as of M5 the driver runs
  the full pipeline (lex → parse → lower → verify → interpret) and executes
  programs. See the Resolved items section.

### TD-0003 — Single MIT license
- **Severity:** low
- **Context.** The project is MIT-licensed for simplicity (ADR-0007).
- **Impact.** The Rust-ecosystem norm is dual MIT/Apache-2.0 (the Apache half adds
  an explicit patent grant).
- **Trigger.** Before any significant external contribution or public release,
  decide whether to dual-license; if so, add `LICENSE-APACHE`, update
  `license = "MIT OR Apache-2.0"`, and note it in `CONTRIBUTING.md`.

### TD-0004 — CI runs a single toolchain on a single OS
- **Severity:** low
- **Context.** CI builds/tests with the pinned stable toolchain on
  `ubuntu-latest` only.
- **Impact.** No coverage of macOS/Windows or of the declared MSRV (1.89) drifting
  from the pinned toolchain.
- **Trigger.** When the platform stabilizes, expand to an OS matrix and add an
  explicit MSRV job.

### TD-0005 — CI lint gating relies on `RUSTFLAGS`-free clippy step
- **Severity:** low
- **Context.** Lints are gated by the `clippy -- -D warnings` step; build/test do
  not force `-D warnings` (so future dependency warnings do not fail CI).
- **Impact.** A rustc-only warning that clippy somehow does not surface would not
  fail the build/test steps.
- **Trigger.** Revisit if we observe warnings escaping the clippy gate; consider a
  scoped deny via `[lints]` levels instead of a global flag.

### TD-0006 — `Span` is 12 bytes; rustc-style packing deferred
- **Severity:** low
- **Context.** `Span { file: FileId, lo, hi }` uses three `u32`s (ADR-0008). rustc
  packs file+range into 4 bytes (inline for small spans, interned otherwise).
- **Impact.** More memory per token/AST/IR node than a packed scheme.
- **Trigger.** If span storage shows up in profiling once real programs compile,
  pack the representation. `Span`'s private fields make this a non-breaking change.

### TD-0007 — Diagnostic renderer: single-line, ungrouped
- **Severity:** low
- **Context.** `render` underlines only the first line of a multi-line span and
  renders one snippet block per label (labels on the same line are not grouped).
- **Impact.** Multi-line spans and multi-label-per-line diagnostics look less
  polished than rustc's.
- **Trigger.** When diagnostics commonly span multiple lines (e.g. block/type
  errors), add multi-line rendering and per-line label grouping.

### TD-0008 — Columns count characters, not display width
- **Severity:** low
- **Context.** A column is one Unicode scalar value; double-width (CJK) and
  zero-width glyphs, and tab display width, are not accounted for in the reported
  column number.
- **Impact.** Caret alignment can drift for such glyphs (tabs are handled in the
  caret pad, but not in reported columns).
- **Trigger.** Add Unicode-width-aware column computation if/when it matters for
  users; consider the `unicode-width` crate.

### TD-0009 — No colored diagnostic output
- **Severity:** low
- **Context.** `render` emits plain text so output is stable to snapshot-test and
  safe to pipe.
- **Impact.** Less scannable in a terminal than colored output.
- **Trigger.** Add ANSI coloring gated on stderr being a TTY (and a `--color`
  flag), keeping a plain-text path for tests.

### TD-0010 — No `Session`/context type yet
- **Severity:** low
- **Context.** `ARCHITECTURE.md` envisions a `Session` owning per-compilation
  state (source map, diagnostics, interners, config). Today the driver holds a
  `SourceMap` and `DiagnosticHandler` directly.
- **Impact.** None yet; introducing it now would be premature abstraction.
- **Trigger.** Introduce `Session` when a third shared component (e.g. a string
  interner) and multiple phases need to thread the same state. The payload-free
  token design (ADR-0010) means M2 did not need it; reassess around name
  resolution (M7).

### TD-0011 — ASCII-only identifiers
- **Severity:** low
- **Context.** The lexer accepts `[A-Za-z_][A-Za-z0-9_]*`; non-ASCII letters are
  rejected as unexpected characters.
- **Impact.** No Unicode identifiers.
- **Trigger.** If Unicode identifiers become desirable, adopt UAX #31 (XID_Start /
  XID_Continue), e.g. via the `unicode-xid` crate.

### TD-0012 — Integer literals are minimal
- **Severity:** low
- **Context.** An integer literal is a maximal run of ASCII digits. There is no
  support for underscores (`1_000`), non-decimal bases (`0x`, `0b`, `0o`), and no
  value/overflow checking (values are parsed later, per ADR-0010). `12ab` lexes as
  an `Int` followed by an `Ident` rather than a malformed-number error.
- **Impact.** Limited literal syntax; some malformed numbers are not flagged at
  lex time.
- **Trigger.** Extend literal syntax and add overflow diagnostics when the type
  system/parser handle integer values (M3+).

### TD-0013 — No block comments
- **Severity:** low
- **Context.** Only `//` line comments are recognized; `/* ... */` is not.
- **Impact.** No block or doc comments.
- **Trigger.** Add (nesting-aware) block comments when needed; decide on doc-comment
  syntax at the same time.

### TD-0014 — No diagnostic error-code registry
- **Severity:** low
- **Context.** Diagnostics support an optional code, but there is no central
  registry, so lexer errors currently carry none.
- **Impact.** No stable, documented error codes for users to look up.
- **Trigger.** Introduce an error-code registry (with explanations) once there are
  enough diagnostics across phases to warrant it.

### TD-0015 — Lexer peek re-slices the source each call
- **Severity:** low
- **Context.** `peek`/`peek_second` do `self.src[self.pos..].chars().next()/nth(1)`,
  re-slicing and decoding on every lookahead.
- **Impact.** Redundant work (still `O(1)` per call); negligible at current scale.
- **Trigger.** If lexing shows up in profiling, cache a `Chars`/`CharIndices`
  iterator or a small lookahead buffer.

### TD-0016 — Parser error recovery is basic
- **Severity:** low
- **Context.** `parse_fn` aborts on the first missing token (via `?`) and recovery
  synchronizes only to the next `fn`. Statement-level recovery is a
  progress-guard-plus-error, not targeted resynchronization.
- **Impact.** A single malformed function can discard the rest of that function
  and occasionally produce a slightly redundant diagnostic (e.g. an unclosed
  paren yielding both "expected expression" and "expected `)`").
- **Trigger.** Improve recovery (statement/expression-level synchronization,
  recovery sets) as the grammar grows and diagnostic quality matters more.

### TD-0017 — No parser recursion-depth guard
- **Severity:** low
- **Context.** Expression parsing recurses; the AST is a `Box` tree (ADR-0011).
  Pathologically nested input (e.g. thousands of `(((…)))`) could overflow the
  stack during parsing or drop.
- **Impact.** A crafted input could crash the compiler with a stack overflow.
- **Trigger.** Add a configurable nesting-depth limit that emits a diagnostic
  before the stack is exhausted; relevant once untrusted input is a concern.

### TD-0018 — No function parameters
- **Severity:** low
- **Context.** The grammar accepts only an empty parameter list `()`; parameter
  syntax (`name: type`) needs a `:` token the lexer does not yet have.
- **Impact.** Functions cannot take arguments.
- **Trigger.** Add the `:` token and parameter parsing (with an AST `Param` type)
  during language expansion (M6).

### TD-0019 — AIR is single-block; no phi / block parameters
- **Severity:** medium
- **Context.** AIR functions currently have one basic block (straight-line code).
  The CFG shape exists, but multiple blocks, branch terminators, and SSA merges
  (phi nodes or block parameters) are not implemented, and the phi-vs-block-params
  choice is deliberately deferred (ADR-0013).
- **Impact.** No control flow can be represented yet.
- **Trigger.** Introduce with control-flow language features (M6). Decide
  phi-vs-block-parameters then, and extend the verifier to dominance-based
  def-before-use across blocks.

### TD-0020 — Missing `return` surfaces as an AIR verification error
- **Severity:** medium
- **Context.** With no semantic analysis yet, a function that omits `return`
  lowers to an unterminated block and is caught by the AIR verifier
  ("block0 has no terminator") rather than by a friendly source-level diagnostic.
- **Impact.** The message is IR-jargon, not a clear "missing return" error; and it
  blurs the verifier's role (which should catch compiler bugs, not user errors).
- **Trigger.** Semantic analysis (M8) should report missing/!-returning functions
  with a proper diagnostic; verification then reverts to an internal-invariant
  check (possibly debug-only).

### TD-0021 — Literal value vs type range is unchecked in lowering
- **Severity:** low
- **Context.** `lower` casts the parser's `u64` literal to `i64` (`as i64`) and
  `lower_type` maps any type name to `int`. Neither range (does the literal fit the
  target type?) nor type-name validity (is `foo` a real type?) is checked.
- **Impact.** An out-of-range literal wraps silently; an unknown return type is
  silently treated as `int`.
- **Trigger.** The type system (M8) validates type names and literal ranges.

### TD-0022 — No AIR textual parser (printer only)
- **Severity:** low
- **Context.** AIR has a textual printer but no parser, so the textual form is not
  yet round-trippable.
- **Impact.** `FileCheck`-style tests that author AIR by hand are not possible;
  golden tests print AIR built via the API instead.
- **Trigger.** Add a textual AIR parser if/when hand-written IR tests or an
  `aetherc` "assemble AIR" entry point are wanted.

### TD-0023 — Interpreter executes only the entry block
- **Severity:** medium
- **Context.** `run_function` evaluates the entry block and acts on its
  terminator; it does not follow branches between blocks (there are none yet).
- **Impact.** No control flow can be executed. Pairs with TD-0019 (single-block
  AIR).
- **Trigger.** Extend to CFG execution (a work-list / successor-following loop,
  and phi/block-parameter handling) alongside control-flow language features (M6).

### TD-0024 — Runtime values are `i64` only
- **Severity:** low
- **Context.** The interpreter represents every runtime value as `i64`; there is
  no runtime value enum because `int` is the only type.
- **Impact.** Cannot represent other types (booleans, etc.).
- **Trigger.** Introduce a runtime `Value` enum when the type system adds more
  types (M8).

### TD-0025 — Overflow policy is provisional (wrapping)
- **Severity:** low
- **Context.** Per ADR-0015 the interpreter wraps on integer overflow. This is a
  provisional interpreter choice, not a ratified language semantics.
- **Impact.** The language has no defined/checked overflow behavior yet.
- **Trigger.** Decide the language's overflow policy (wrap / checked / saturating,
  possibly per-type or per-operator) with the type system (M8) and align the
  interpreter and any future backend with it.

---

## Resolved items

- **TD-0002 — `aetherc` compilation pipeline is a stub.** Resolved in M5: the
  driver runs the full pipeline (lex → parse → lower → verify → interpret) and
  executes programs, printing `main`'s result. Complete for the minimal language.
