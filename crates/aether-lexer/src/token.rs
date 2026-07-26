//! Token definitions: [`TokenKind`] and [`Token`].

use aether_source::Span;

/// The lexical category of a [`Token`].
///
/// This enum is intentionally **payload-free**: variants like [`TokenKind::Ident`]
/// and [`TokenKind::Int`] carry no text or value. The lexeme is recovered from the
/// source via the token's [`Span`] when needed (see the crate-level docs and
/// ADR-0010). This keeps `TokenKind` `Copy` and cheap to match on.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TokenKind {
    /// An identifier, e.g. `main` or `int` (type names are identifiers for now).
    Ident,
    /// A decimal integer literal, e.g. `42`.
    Int,

    /// The `fn` keyword.
    Fn,
    /// The `return` keyword.
    Return,
    /// The `let` keyword.
    Let,
    /// The `true` boolean literal keyword.
    True,
    /// The `false` boolean literal keyword.
    False,
    /// The `if` keyword.
    If,
    /// The `else` keyword.
    Else,

    /// A left parenthesis `(`.
    LParen,
    /// A right parenthesis `)`.
    RParen,
    /// A left brace `{`.
    LBrace,
    /// A right brace `}`.
    RBrace,

    /// A semicolon `;`.
    Semicolon,
    /// A colon `:`.
    Colon,
    /// A comma `,`.
    Comma,

    /// A plus sign `+`.
    Plus,
    /// A minus sign `-`.
    Minus,
    /// An asterisk `*`.
    Star,
    /// A forward slash `/`.
    Slash,
    /// An arrow `->`.
    Arrow,
    /// An equals sign `=`.
    Eq,

    /// A double-equals `==` (equality).
    EqEq,
    /// A `!=` (inequality).
    BangEq,
    /// A less-than `<`.
    Lt,
    /// A less-than-or-equal `<=`.
    LtEq,
    /// A greater-than `>`.
    Gt,
    /// A greater-than-or-equal `>=`.
    GtEq,
    /// A logical-not `!`.
    Bang,
    /// A logical-and `&&`.
    AmpAmp,
    /// A logical-or `||`.
    PipePipe,

    /// The end of the input. Always the final token; has an empty span.
    Eof,
}

impl TokenKind {
    /// A short human-readable description, suitable for diagnostics
    /// (e.g. `` "`+`" `` or `"identifier"`).
    #[must_use]
    pub fn description(self) -> &'static str {
        use TokenKind::*;
        match self {
            Ident => "identifier",
            Int => "integer literal",
            Fn => "keyword `fn`",
            Return => "keyword `return`",
            Let => "keyword `let`",
            True => "keyword `true`",
            False => "keyword `false`",
            If => "keyword `if`",
            Else => "keyword `else`",
            LParen => "`(`",
            RParen => "`)`",
            LBrace => "`{`",
            RBrace => "`}`",
            Semicolon => "`;`",
            Colon => "`:`",
            Comma => "`,`",
            Plus => "`+`",
            Minus => "`-`",
            Star => "`*`",
            Slash => "`/`",
            Arrow => "`->`",
            Eq => "`=`",
            EqEq => "`==`",
            BangEq => "`!=`",
            Lt => "`<`",
            LtEq => "`<=`",
            Gt => "`>`",
            GtEq => "`>=`",
            Bang => "`!`",
            AmpAmp => "`&&`",
            PipePipe => "`||`",
            Eof => "end of file",
        }
    }

    /// Whether this kind is a reserved keyword.
    #[must_use]
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            TokenKind::Fn
                | TokenKind::Return
                | TokenKind::Let
                | TokenKind::True
                | TokenKind::False
                | TokenKind::If
                | TokenKind::Else
        )
    }
}

/// If `lexeme` is a reserved keyword, return its [`TokenKind`]; otherwise `None`
/// (the lexeme is an ordinary identifier).
pub(crate) fn keyword(lexeme: &str) -> Option<TokenKind> {
    match lexeme {
        "fn" => Some(TokenKind::Fn),
        "return" => Some(TokenKind::Return),
        "let" => Some(TokenKind::Let),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        _ => None,
    }
}

/// A lexical token: a [`TokenKind`] plus the source [`Span`] it occupies.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Token {
    /// The lexical category of the token.
    pub kind: TokenKind,
    /// The source region the token covers.
    pub span: Span,
}

impl Token {
    /// Create a token from a kind and span.
    #[must_use]
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_are_recognized() {
        assert_eq!(keyword("fn"), Some(TokenKind::Fn));
        assert_eq!(keyword("return"), Some(TokenKind::Return));
        assert_eq!(keyword("let"), Some(TokenKind::Let));
        assert_eq!(keyword("true"), Some(TokenKind::True));
        assert_eq!(keyword("false"), Some(TokenKind::False));
        assert_eq!(keyword("if"), Some(TokenKind::If));
        assert_eq!(keyword("else"), Some(TokenKind::Else));
    }

    #[test]
    fn non_keywords_are_none() {
        assert_eq!(keyword("main"), None);
        assert_eq!(keyword("int"), None); // type names are identifiers for now
        assert_eq!(keyword("fna"), None);
        assert_eq!(keyword(""), None);
    }

    #[test]
    fn keyword_predicate_matches_lookup() {
        assert!(TokenKind::Fn.is_keyword());
        assert!(TokenKind::Return.is_keyword());
        assert!(TokenKind::True.is_keyword());
        assert!(TokenKind::False.is_keyword());
        assert!(!TokenKind::Ident.is_keyword());
        assert!(!TokenKind::Plus.is_keyword());
    }
}
