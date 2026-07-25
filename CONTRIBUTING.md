# Contributing to Aether

Aether is built as if it were a large open-source project with many future
contributors. These standards keep it maintainable at scale. They apply to every
change, including the author's own.

---

## Golden rules

1. **Architecture before implementation.** For any non-trivial subsystem: explain
   the theory, weigh alternatives, record the decision (an ADR in
   [`DECISIONS.md`](DECISIONS.md)), then implement.
2. **The repository is the source of truth.** Keep the project documents current;
   never rely on out-of-band memory or chat history.
3. **No premature abstraction and no placeholder crates.** Introduce a crate,
   trait, or generalization only when a real consumer needs it.
4. **Leave it green.** Every change builds, lints cleanly, and passes tests.

---

## Session workflow

Each working session targets **exactly one milestone** (see
[`ROADMAP.md`](ROADMAP.md)).

- **Start:** read [`PROJECT_STATUS.md`](PROJECT_STATUS.md), confirm the current
  milestone and that prerequisites are done, then plan.
- **End:** code complete and tested; documentation and the project-management
  documents updated; technical debt recorded in [`TECH_DEBT.md`](TECH_DEBT.md);
  and exactly one logical next milestone recommended in `PROJECT_STATUS.md`.

---

## Coding standards

### Formatting & lints (enforced by CI)
- `cargo fmt --all` — formatting is not negotiable; CI runs `--check`.
- `cargo clippy --all-targets -- -D warnings` must be clean. The workspace lint
  policy lives in the root `Cargo.toml` and applies to every crate via
  `[lints] workspace = true`.
- `unsafe` is denied workspace-wide. A crate that genuinely needs it (e.g. a
  future codegen backend) may downgrade the lint locally **with a documented
  rationale and a focused, well-commented `unsafe` block**.

### Documentation
- Public items are documented; `missing_docs` is enabled. Write module-level docs
  explaining each crate's purpose and responsibilities.
- Explain *why*, not just *what*. Design rationale belongs in `DECISIONS.md`;
  local rationale belongs in comments near the code.

### Errors & robustness
- Library code returns `Result` with meaningful error types; it does not `panic!`,
  `unwrap`, or `expect` on conditions reachable from user input. `unwrap`/`expect`
  are acceptable in tests and on genuine invariants (with a message explaining the
  invariant).
- User-facing problems are reported through the diagnostics system, not printed
  ad hoc.

### Testing
- Every subsystem is tested. Prefer unit tests colocated in a `#[cfg(test)]`
  module for internal logic, and integration tests under `tests/` for
  crate-boundary and end-to-end behavior.
- Once AIR has a textual form, prefer golden/round-trip tests for IR and
  transformations.
- Add a test with every bug fix that reproduces the bug.

### Module & crate organization
- One subsystem per crate; keep dependencies flowing frontend → IR → backend with
  foundational crates at the bottom (see [`ARCHITECTURE.md`](ARCHITECTURE.md) §3).
  No dependency cycles.

---

## Commits

- Small, focused, logically atomic commits with imperative-mood subjects
  (e.g. "Add span-to-line/column mapping to aether-source").
- Explain *why* in the body when it is not obvious.
- Every commit should build and pass tests.

---

## Toolchain

The toolchain is pinned in [`rust-toolchain.toml`](rust-toolchain.toml) and
installs automatically on first `cargo` use. The declared MSRV is recorded as
`rust-version` in the workspace `Cargo.toml`.

---

## Before opening a change

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo build --all-targets
cargo test --all
```

All four must pass — exactly what CI checks.
