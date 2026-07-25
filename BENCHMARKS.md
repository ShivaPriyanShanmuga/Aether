# Aether — Benchmarks

Performance history and methodology. The governing rule is **measure before
optimizing**: performance claims and optimization work must be backed by numbers
recorded here, not by intuition.

---

## Status

No benchmarks yet. There is nothing meaningful to measure until the compilation
pipeline exists (Phase 1) and, for optimization work, until the analysis and
optimization frameworks land (Phase 3). This document defines the methodology now
so that results are recorded consistently once there is something to measure.

---

## Methodology (to apply once benchmarks exist)

- **Harness.** Use [`criterion`](https://crates.io/crates/criterion) for
  microbenchmarks (statistically rigorous, tracks regressions). Add a dedicated
  `benches/` target in the relevant crate; wire a `cargo bench` entry point. For
  end-to-end compile-time measurements, use a small corpus of representative `.ae`
  programs.
- **What to measure.** Distinguish and record separately: compiler throughput
  (lines or bytes per second per phase), peak memory, and — once a backend exists
  — the runtime/quality of generated code. Attribute time per phase so regressions
  are localizable.
- **Environment.** Every recorded result must state: CPU, OS, Rust toolchain
  version, build profile (`--release` unless noted), and input corpus + size.
  Numbers without an environment are not comparable and should not be recorded.
- **Regression policy.** Note the baseline a change is compared against. A
  performance-motivated change should show a before/after with the same
  environment and inputs.

---

## History

| Date | Commit | Scenario | Metric | Result | Environment | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| — | — | — | — | — | — | No benchmarks recorded yet |
