# Aether

**A modular, production-quality compiler platform**, built in Rust and inspired by
LLVM, Clang, rustc, GCC, MLIR, and Cranelift.

Aether is not a toy or tutorial compiler. It is an engineering project aimed at a
maintainable, extensible, well-tested compiler *platform* — a frontend, a custom
intermediate representation (**AIR**), an optimization and analysis framework, and
pluggable backends — designed so it can keep growing for years.

> **Status:** early, but **it runs**. Phase 1 (First Light) is complete: a program
> compiles and executes end to end — lexer → parser → AST → AIR (the custom IR) →
> interpreter. The language today is minimal: one `main` function returning an
> integer arithmetic expression. See [`PROJECT_STATUS.md`](PROJECT_STATUS.md) for
> exactly where things stand and [`ROADMAP.md`](ROADMAP.md) for where they go next
> (local variables, control flow, functions).

## Quickstart

```sh
# Build everything
cargo build

# Write and run a tiny program
echo 'fn main() -> int { return (10 - 4) * 7 + -2; }' > demo.ae
cargo run -q -p aetherc -- demo.ae            # prints: 40

# Inspect intermediate stages
cargo run -q -p aetherc -- --dump-tokens demo.ae
cargo run -q -p aetherc -- --dump-ast demo.ae
cargo run -q -p aetherc -- --dump-air demo.ae
cargo run -p aetherc -- --help

# Run the full test suite (unit + integration)
cargo test

# Lint and format exactly as CI does
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

The pinned toolchain (see [`rust-toolchain.toml`](rust-toolchain.toml)) installs
automatically on first `cargo` invocation.

## Repository layout

```
Aether/
├── crates/
│   ├── aetherc/            # command-line driver (entry point)
│   ├── aether-source/      # source files, spans, line/column
│   ├── aether-diagnostics/ # structured diagnostics + caret rendering
│   ├── aether-lexer/       # tokenizer
│   ├── aether-ast/         # AST + pretty-printer
│   ├── aether-parser/      # recursive-descent + Pratt parser
│   ├── aether-air/         # AIR: typed SSA IR + printer + verifier
│   ├── aether-lower/       # AST → AIR lowering
│   └── aether-air-interp/  # AIR interpreter
├── .github/workflows/      # CI
└── *.md                    # project-management documents (see below)
```

The full planned crate graph (optimizer, analyses, backends, …) is
documented in [`ARCHITECTURE.md`](ARCHITECTURE.md). Crates are created only when
something depends on them, so the tree grows one milestone at a time.

## Project documents

This repository is the single source of truth. Before starting work, read:

| Document | Purpose |
| --- | --- |
| [`PROJECT_STATUS.md`](PROJECT_STATUS.md) | Current milestone, progress, what's next |
| [`ROADMAP.md`](ROADMAP.md) | Long-term phases and milestones |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | System architecture and design principles |
| [`DECISIONS.md`](DECISIONS.md) | Architecture Decision Records (ADRs) |
| [`TECH_DEBT.md`](TECH_DEBT.md) | Known limitations and planned refactors |
| [`BENCHMARKS.md`](BENCHMARKS.md) | Performance methodology and history |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Coding standards and workflow |

## License

Licensed under the [MIT License](LICENSE).
