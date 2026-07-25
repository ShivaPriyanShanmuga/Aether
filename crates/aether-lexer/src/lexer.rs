//! The scanner: a hand-written, character-based lexer with error recovery.

use aether_diagnostics::Diagnostic;
use aether_source::{BytePos, FileId, SourceFile, Span};

use crate::token::{Token, TokenKind, keyword};

/// The result of lexing a source file: the token stream and any lexical
/// diagnostics produced along the way.
///
/// The token stream always ends with a [`TokenKind::Eof`] token, even when
/// diagnostics were produced, so consumers (the parser) can rely on a terminator.
#[derive(Debug)]
pub struct LexResult {
    /// The tokens, in source order, terminated by [`TokenKind::Eof`].
    pub tokens: Vec<Token>,
    /// Diagnostics emitted during lexing (e.g. unexpected characters).
    pub diagnostics: Vec<Diagnostic>,
}

/// Tokenize a source file into a [`LexResult`].
#[must_use]
pub fn tokenize(file: &SourceFile) -> LexResult {
    Lexer::new(file.id(), file.source()).run()
}

/// Internal scanner state. Walks the source one character at a time, tracking the
/// current byte offset so that every token gets an accurate [`Span`].
struct Lexer<'src> {
    file: FileId,
    src: &'src str,
    /// Current byte offset into `src`.
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Lexer<'src> {
    fn new(file: FileId, src: &'src str) -> Lexer<'src> {
        Lexer {
            file,
            src,
            pos: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn run(mut self) -> LexResult {
        loop {
            self.skip_trivia();
            let start = self.pos;
            match self.peek() {
                None => {
                    // Terminate with an empty end-of-file token.
                    self.push(TokenKind::Eof, start, start);
                    break;
                }
                Some(c) if c.is_ascii_digit() => self.number(start),
                Some(c) if is_ident_start(c) => self.ident(start),
                Some(c) => self.symbol_or_error(start, c),
            }
        }

        LexResult {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    /// Skip whitespace and line comments between tokens.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                // A `//` line comment runs to the end of the line.
                Some('/') if self.peek_second() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    /// Scan a decimal integer literal (a maximal run of ASCII digits).
    fn number(&mut self, start: usize) {
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        self.push(TokenKind::Int, start, self.pos);
    }

    /// Scan an identifier or keyword.
    fn ident(&mut self, start: usize) {
        while matches!(self.peek(), Some(c) if is_ident_continue(c)) {
            self.bump();
        }
        let lexeme = &self.src[start..self.pos];
        let kind = keyword(lexeme).unwrap_or(TokenKind::Ident);
        self.push(kind, start, self.pos);
    }

    /// Scan an operator/delimiter, or report an unexpected character.
    fn symbol_or_error(&mut self, start: usize, c: char) {
        self.bump(); // consume the first character
        let kind = match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            // `/` line comments are handled by `skip_trivia`, so a `/` here is division.
            '/' => TokenKind::Slash,
            // Two-character operators share a first character with a one-character
            // one, so each peeks ahead for the trailing `=`/`>` (like `->`).
            '=' => self.one_or_two('=', TokenKind::EqEq, TokenKind::Eq),
            '!' => self.one_or_two('=', TokenKind::BangEq, TokenKind::Bang),
            '<' => self.one_or_two('=', TokenKind::LtEq, TokenKind::Lt),
            '>' => self.one_or_two('=', TokenKind::GtEq, TokenKind::Gt),
            '-' => self.one_or_two('>', TokenKind::Arrow, TokenKind::Minus),
            _ => {
                let span = self.span(start, self.pos);
                self.diagnostics.push(
                    Diagnostic::error(format!("unexpected character {c:?}"))
                        .with_primary(span, "unknown start of a token"),
                );
                return;
            }
        };
        self.push(kind, start, self.pos);
    }

    /// Resolve a one- or two-character operator: if the next character is
    /// `second`, consume it and return `two`; otherwise return `one`. The first
    /// character has already been consumed by the caller.
    fn one_or_two(&mut self, second: char, two: TokenKind, one: TokenKind) -> TokenKind {
        if self.peek() == Some(second) {
            self.bump();
            two
        } else {
            one
        }
    }

    /// The character at the current position, without consuming it.
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    /// The character one position ahead, without consuming anything.
    fn peek_second(&self) -> Option<char> {
        self.src[self.pos..].chars().nth(1)
    }

    /// Consume and return the current character, advancing by its UTF-8 length.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(
            self.file,
            BytePos::from_usize(start),
            BytePos::from_usize(end),
        )
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        let span = self.span(start, end);
        self.tokens.push(Token::new(kind, span));
    }
}

/// Whether `c` may begin an identifier (ASCII letter or underscore).
///
/// Unicode identifiers are a deliberate future extension (see `TECH_DEBT.md`).
fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

/// Whether `c` may continue an identifier (ASCII alphanumeric or underscore).
fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::SourceMap;

    /// Tokenize `src` and return `(kind, lexeme)` pairs plus any diagnostics.
    fn lex(src: &str) -> (Vec<(TokenKind, String)>, Vec<Diagnostic>) {
        let mut map = SourceMap::new();
        let id = map.add_file("test.ae", src);
        let result = tokenize(map.file(id));
        let pairs = result
            .tokens
            .iter()
            .map(|t| (t.kind, map.span_text(t.span).to_string()))
            .collect();
        (pairs, result.diagnostics)
    }

    /// Tokenize and return just the kinds (dropping the trailing `Eof`).
    fn kinds(src: &str) -> Vec<TokenKind> {
        let (pairs, diags) = lex(src);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let mut ks: Vec<TokenKind> = pairs.into_iter().map(|(k, _)| k).collect();
        assert_eq!(ks.pop(), Some(TokenKind::Eof), "stream must end with Eof");
        ks
    }

    #[test]
    fn empty_input_is_just_eof() {
        let (pairs, diags) = lex("");
        assert!(diags.is_empty());
        assert_eq!(pairs, vec![(TokenKind::Eof, String::new())]);
    }

    #[test]
    fn whitespace_only_is_just_eof() {
        let (pairs, diags) = lex("  \t\n  ");
        assert!(diags.is_empty());
        assert_eq!(pairs, vec![(TokenKind::Eof, String::new())]);
    }

    #[test]
    fn keywords_versus_identifiers() {
        let (pairs, _) = lex("fn foo return int");
        assert_eq!(
            pairs,
            vec![
                (TokenKind::Fn, "fn".to_string()),
                (TokenKind::Ident, "foo".to_string()),
                (TokenKind::Return, "return".to_string()),
                (TokenKind::Ident, "int".to_string()), // `int` is an identifier
                (TokenKind::Eof, String::new()),
            ]
        );
    }

    #[test]
    fn integer_literals() {
        let (pairs, diags) = lex("0 42 007");
        assert!(diags.is_empty());
        assert_eq!(
            pairs,
            vec![
                (TokenKind::Int, "0".to_string()),
                (TokenKind::Int, "42".to_string()),
                (TokenKind::Int, "007".to_string()),
                (TokenKind::Eof, String::new()),
            ]
        );
    }

    #[test]
    fn number_then_identifier_split_at_boundary() {
        // `12ab` lexes as an integer immediately followed by an identifier.
        let (pairs, _) = lex("12ab");
        assert_eq!(
            pairs,
            vec![
                (TokenKind::Int, "12".to_string()),
                (TokenKind::Ident, "ab".to_string()),
                (TokenKind::Eof, String::new()),
            ]
        );
    }

    #[test]
    fn all_operators_and_delimiters() {
        assert_eq!(
            kinds("( ) { } ; , + - * /"),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Semicolon,
                TokenKind::Comma,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
            ]
        );
    }

    #[test]
    fn arrow_versus_minus() {
        assert_eq!(kinds("->"), vec![TokenKind::Arrow]);
        assert_eq!(kinds("-"), vec![TokenKind::Minus]);
        assert_eq!(kinds("- ->"), vec![TokenKind::Minus, TokenKind::Arrow]);
        // Adjacent, no spaces: `a-b` is ident, minus, ident.
        assert_eq!(
            kinds("a-b"),
            vec![TokenKind::Ident, TokenKind::Minus, TokenKind::Ident]
        );
    }

    #[test]
    fn line_comments_are_skipped() {
        let (pairs, diags) = lex("1 // this is a comment\n2");
        assert!(diags.is_empty());
        assert_eq!(
            pairs,
            vec![
                (TokenKind::Int, "1".to_string()),
                (TokenKind::Int, "2".to_string()),
                (TokenKind::Eof, String::new()),
            ]
        );
    }

    #[test]
    fn line_comment_at_end_of_file() {
        let (pairs, diags) = lex("42 // trailing comment with no newline");
        assert!(diags.is_empty());
        assert_eq!(
            pairs,
            vec![
                (TokenKind::Int, "42".to_string()),
                (TokenKind::Eof, String::new()),
            ]
        );
    }

    #[test]
    fn let_binding_tokens() {
        assert_eq!(
            kinds("let x = 5"),
            vec![
                TokenKind::Let,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Int,
            ]
        );
    }

    #[test]
    fn slash_is_division_not_comment() {
        assert_eq!(
            kinds("1 / 2"),
            vec![TokenKind::Int, TokenKind::Slash, TokenKind::Int]
        );
    }

    #[test]
    fn comparison_and_logical_operators() {
        assert_eq!(
            kinds("== != < <= > >= !"),
            vec![
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::Lt,
                TokenKind::LtEq,
                TokenKind::Gt,
                TokenKind::GtEq,
                TokenKind::Bang,
            ]
        );
    }

    #[test]
    fn one_versus_two_character_operators() {
        // A single `=` is assignment; `==` is equality (two-char lookahead).
        assert_eq!(kinds("="), vec![TokenKind::Eq]);
        assert_eq!(kinds("=="), vec![TokenKind::EqEq]);
        // `!` alone versus `!=`.
        assert_eq!(kinds("!"), vec![TokenKind::Bang]);
        assert_eq!(kinds("!="), vec![TokenKind::BangEq]);
        // `<`/`>` alone versus their `=`-suffixed forms.
        assert_eq!(kinds("< <="), vec![TokenKind::Lt, TokenKind::LtEq]);
        assert_eq!(kinds("> >="), vec![TokenKind::Gt, TokenKind::GtEq]);
        // No greedy merge across whitespace: `= =` is two `=` tokens.
        assert_eq!(kinds("= ="), vec![TokenKind::Eq, TokenKind::Eq]);
    }

    #[test]
    fn boolean_keywords() {
        assert_eq!(kinds("true false"), vec![TokenKind::True, TokenKind::False]);
        // `truer` is an identifier, not the `true` keyword followed by `r`.
        assert_eq!(kinds("truer"), vec![TokenKind::Ident]);
    }

    #[test]
    fn spans_track_byte_offsets() {
        let mut map = SourceMap::new();
        let id = map.add_file("test.ae", "  fn  x");
        let result = tokenize(map.file(id));
        // `fn` at bytes 2..4, `x` at byte 6..7, then Eof at 7..7.
        assert_eq!(result.tokens[0].kind, TokenKind::Fn);
        assert_eq!(result.tokens[0].span.lo(), BytePos(2));
        assert_eq!(result.tokens[0].span.hi(), BytePos(4));
        assert_eq!(result.tokens[1].kind, TokenKind::Ident);
        assert_eq!(result.tokens[1].span.lo(), BytePos(6));
        assert_eq!(result.tokens[1].span.hi(), BytePos(7));
        let eof = result.tokens.last().unwrap();
        assert_eq!(eof.kind, TokenKind::Eof);
        assert!(eof.span.is_empty());
        assert_eq!(eof.span.lo(), BytePos(7));
    }

    #[test]
    fn full_minimal_program() {
        assert_eq!(
            kinds("fn main() -> int { return 1 + 2; }"),
            vec![
                TokenKind::Fn,
                TokenKind::Ident, // main
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Ident, // int
                TokenKind::LBrace,
                TokenKind::Return,
                TokenKind::Int, // 1
                TokenKind::Plus,
                TokenKind::Int, // 2
                TokenKind::Semicolon,
                TokenKind::RBrace,
            ]
        );
    }

    #[test]
    fn unexpected_character_is_reported_and_recovered() {
        // The `@` is invalid, but lexing continues and still yields the `1`.
        let (pairs, diags) = lex("1 @ 2");
        assert_eq!(diags.len(), 1);
        assert_eq!(
            pairs,
            vec![
                (TokenKind::Int, "1".to_string()),
                (TokenKind::Int, "2".to_string()),
                (TokenKind::Eof, String::new()),
            ]
        );
    }

    #[test]
    fn unexpected_multibyte_character_spans_whole_char() {
        // `é` is two bytes; the diagnostic's span must cover both.
        let (_pairs, diags) = lex("é");
        assert_eq!(diags.len(), 1);
        let span = diags[0].primary_span().expect("a primary label");
        assert_eq!(span.lo(), BytePos(0));
        assert_eq!(span.hi(), BytePos(2));
    }
}
