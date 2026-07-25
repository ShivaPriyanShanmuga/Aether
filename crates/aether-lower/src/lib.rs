//! Lowering from the Aether AST ([`aether_ast`]) to AIR ([`aether_air`]).
//!
//! This crate is the bridge between the frontend and the middle end. Keeping it
//! separate lets AIR stay independent of the AST: `aether-air` knows nothing about
//! syntax, and this crate maps AST constructs onto AIR ones.
//!
//! Because the current expression language is a pure tree, lowering is a
//! straightforward post-order walk that emits one AIR instruction per AST node,
//! naturally producing SSA. The result is verified by
//! [`aether_air::verify`](aether_air::verify) downstream.
//!
//! ```
//! # use aether_source::SourceMap;
//! let mut sources = SourceMap::new();
//! let file = sources.add_file("demo.ae", "fn main() -> int { return 1 + 2; }");
//! let tokens = aether_lexer::tokenize(sources.file(file)).tokens;
//! let program = aether_parser::parse(sources.file(file), &tokens).program;
//! let module = aether_lower::lower(&program);
//! assert_eq!(module.functions().len(), 1);
//! assert!(aether_air::verify(&module).is_empty());
//! ```

mod lower;

pub use lower::lower;
