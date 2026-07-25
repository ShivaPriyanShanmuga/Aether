//! Lowering from the Aether AST ([`aether_ast`]) to AIR ([`aether_air`]).
//!
//! This crate is the bridge between the frontend and the middle end. Keeping it
//! separate lets AIR stay independent of the AST: `aether-air` knows nothing about
//! syntax, and this crate maps AST constructs onto AIR ones.
//!
//! Lowering is a post-order walk that emits one AIR instruction per expression
//! node, naturally producing SSA. Local variables are handled with a name → value
//! environment: a `let` binds a name to the value of its initializer, and a name
//! reference resolves to that value. Because this resolves identifiers, lowering
//! can fail (e.g. an unknown name) and so returns a [`LowerResult`] carrying both
//! the module and any diagnostics (see ADR-0016). Structural validity of the
//! result is checked separately by [`aether_air::verify`](aether_air::verify).
//!
//! ```
//! # use aether_source::SourceMap;
//! let mut sources = SourceMap::new();
//! let file = sources.add_file("demo.ae", "fn main() -> int { let x = 6; return x * 7; }");
//! let tokens = aether_lexer::tokenize(sources.file(file)).tokens;
//! let program = aether_parser::parse(sources.file(file), &tokens).program;
//! let result = aether_lower::lower(&program);
//! assert!(result.diagnostics.is_empty());
//! assert!(aether_air::verify(&result.module).is_empty());
//! ```

mod lower;

pub use lower::{LowerResult, lower};
