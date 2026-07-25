//! Lexical analysis for the Aether language.
//!
//! The lexer turns raw UTF-8 source into a flat stream of [`Token`]s — the first
//! frontend phase and the first real consumer of [`aether_source`] (every token
//! carries a [`Span`](aether_source::Span)) and [`aether_diagnostics`] (lexical
//! errors are reported as [`Diagnostic`](aether_diagnostics::Diagnostic)s).
//!
//! # Design
//!
//! - **Payload-free tokens.** [`TokenKind`] is a small `Copy` enum with no owned
//!   payloads; the text of an identifier or the digits of an integer literal are
//!   recovered from the source via the token's span. This keeps tokens tiny and
//!   decouples a token's *shape* from its *value* — and means the lexer needs no
//!   string interning (see ADR-0010).
//! - **Error recovery.** An unexpected character is reported as a diagnostic and
//!   scanning continues, so one bad character does not abort lexing.
//! - **Pure transformation.** [`tokenize`] returns a [`LexResult`] containing both
//!   the tokens and any diagnostics; the caller funnels the diagnostics into a
//!   [`DiagnosticHandler`](aether_diagnostics::DiagnosticHandler). This keeps the
//!   lexer trivially testable in isolation.
//!
//! ```
//! # use aether_source::SourceMap;
//! # use aether_lexer::{tokenize, TokenKind};
//! let mut sources = SourceMap::new();
//! let file = sources.add_file("demo.ae", "return 1 + 2;");
//! let result = tokenize(sources.file(file));
//! assert!(result.diagnostics.is_empty());
//! assert_eq!(result.tokens.first().unwrap().kind, TokenKind::Return);
//! assert_eq!(result.tokens.last().unwrap().kind, TokenKind::Eof);
//! ```

mod lexer;
mod token;

pub use lexer::{LexResult, tokenize};
pub use token::{Token, TokenKind};
