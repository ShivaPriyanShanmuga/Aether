//! The parser for the Aether language.
//!
//! Turns the token stream from [`aether_lexer`] into an [`aether_ast`] tree. It
//! is a hand-written recursive-descent parser, with **Pratt (precedence
//! climbing)** for expressions so that operator precedence lives in a
//! binding-power table rather than a function per level (see ADR-0012).
//!
//! Parsing is error-tolerant: a syntax error is reported as a
//! [`Diagnostic`](aether_diagnostics::Diagnostic) and parsing recovers (poison
//! [`Expr::Error`](aether_ast::Expr::Error) nodes and synchronization to the next
//! item/statement), so a single mistake does not cascade. [`parse`] returns both
//! the resulting [`Program`](aether_ast::Program) and any diagnostics.
//!
//! ```
//! # use aether_source::SourceMap;
//! let mut sources = SourceMap::new();
//! let file = sources.add_file("demo.ae", "fn main() -> int { return 1 + 2; }");
//! let tokens = aether_lexer::tokenize(sources.file(file)).tokens;
//! let result = aether_parser::parse(sources.file(file), &tokens);
//! assert!(result.diagnostics.is_empty());
//! assert_eq!(result.program.items.len(), 1);
//! ```

mod parser;

pub use parser::{ParseResult, parse};
