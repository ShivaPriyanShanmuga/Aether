//! A tree-walking interpreter over AIR — Aether's first execution target.
//!
//! Executing AIR is direct because it is SSA: each instruction's result is
//! computed once and stored, keyed by its [`Value`](aether_air::Value); operands
//! read earlier results. The interpreter evaluates a block's instructions in
//! order and then acts on its terminator (`ret` yields the function's result).
//!
//! Semantics (provisional, to be formalized with the type system — see ADR-0015):
//! integer arithmetic **wraps** (two's complement), and **division by zero is a
//! runtime error** carrying the offending source span.
//!
//! The interpreter is decoupled from diagnostics: it returns a [`RunError`] that
//! the caller renders. It assumes the module has passed
//! [`aether_air::verify`](aether_air::verify).
//!
//! ```
//! # use aether_source::SourceMap;
//! let mut sources = SourceMap::new();
//! let file = sources.add_file("demo.ae", "fn main() -> int { return 6 * 7; }");
//! let tokens = aether_lexer::tokenize(sources.file(file)).tokens;
//! let program = aether_parser::parse(sources.file(file), &tokens).program;
//! let module = aether_lower::lower(&program).module;
//! assert_eq!(
//!     aether_air_interp::interpret(&module),
//!     Ok(aether_air_interp::RunValue::Int(42))
//! );
//! ```

mod interp;

pub use interp::{RunError, RunValue, interpret, run_function};
