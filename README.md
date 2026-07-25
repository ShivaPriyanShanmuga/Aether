# Aether

**A modular, production-quality compiler platform**, built in Rust and inspired by
LLVM, Clang, rustc, GCC, MLIR, and Cranelift.

Aether is not a toy or tutorial compiler. It is an engineering project aimed at a
maintainable, extensible, well-tested compiler *platform* — a frontend, a custom
intermediate representation (**AIR**), an optimization and analysis framework, and
pluggable backends — designed so it can keep growing for years.

> **Status:** early. Milestone 0 (project foundation) is complete: workspace,
> tooling, CI, coding standards, and the `aetherc` driver skeleton are in place.
> The compilation pipeline itself is not yet implemented. See
> [`PROJECT_STATUS.md`](PROJECT_STATUS.md) for exactly where things stand and
> [`ROADMAP.md`](ROADMAP.md) for where they are going.

## Quickstart

```sh
# Build everything
cargo build

# Run the driver
cargo run -p aetherc -- --help
cargo run -p aetherc -- --version

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
│   └── aetherc/          # command-line driver (entry point)
├── .github/workflows/    # CI
└── *.md                  # project-management documents (see below)
```

The full planned crate graph (lexer, parser, AIR, optimizer, backends, …) is
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
