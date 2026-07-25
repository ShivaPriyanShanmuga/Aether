//! The recursive-descent parser with Pratt expression parsing.

use aether_ast::{
    BinOp, Block, ElseBranch, Expr, FnDecl, Ident, IfStmt, Item, LetStmt, Program, ReturnStmt,
    Stmt, Type, UnOp,
};
use aether_diagnostics::Diagnostic;
use aether_lexer::{Token, TokenKind};
use aether_source::{SourceFile, Span};

/// The result of parsing: the (possibly partial) program and any diagnostics.
#[derive(Debug)]
pub struct ParseResult {
    /// The parsed program. On errors it is a best-effort partial tree, possibly
    /// containing [`Expr::Error`] nodes.
    pub program: Program,
    /// Diagnostics emitted during parsing.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse `tokens` (produced by lexing `file`) into a [`ParseResult`].
///
/// `tokens` is expected to end with a [`TokenKind::Eof`] token, as the lexer
/// guarantees.
#[must_use]
pub fn parse(file: &SourceFile, tokens: &[Token]) -> ParseResult {
    Parser::new(file, tokens).parse_program()
}

/// Right binding power of the unary prefix operators (`-`, `!`). Higher than any
/// binary operator, so `-a * b` parses as `(-a) * b` and `!a == b` as `(!a) == b`.
const UNARY_BINDING_POWER: u8 = 15;

/// The `(left, right)` binding powers of an infix operator. `right > left` makes
/// the operator left-associative.
///
/// Precedence, loosest to tightest: logical-or, logical-and, equality,
/// relational, additive, multiplicative (unary prefix binds tighter than all of
/// them). Comparison operators are left-associative for now; chained comparisons
/// like `a < b < c` parse but are rejected later by AIR type checking (a nicer
/// non-associativity error awaits the type system).
fn infix_binding_power(op: BinOp) -> (u8, u8) {
    match op {
        BinOp::Or => (1, 2),
        BinOp::And => (3, 4),
        BinOp::Eq | BinOp::Ne => (5, 6),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => (7, 8),
        BinOp::Add | BinOp::Sub => (9, 10),
        BinOp::Mul | BinOp::Div => (11, 12),
    }
}

struct Parser<'a> {
    /// The full source text, for recovering identifier and literal lexemes.
    src: &'a str,
    tokens: &'a [Token],
    /// Index of the current token. Never advances past the final `Eof`.
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(file: &'a SourceFile, tokens: &'a [Token]) -> Parser<'a> {
        Parser {
            src: file.source(),
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    // --- Cursor -------------------------------------------------------------

    fn peek(&self) -> Token {
        // The lexer guarantees a trailing Eof, so `last` is always valid.
        self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    /// Consume and return the current token (never advancing past `Eof`).
    fn bump(&mut self) -> Token {
        let token = self.peek();
        if token.kind != TokenKind::Eof {
            self.pos += 1;
        }
        token
    }

    /// If the current token is `kind`, consume it and return its span; otherwise
    /// emit an "expected" diagnostic and return `None` without consuming.
    fn expect(&mut self, kind: TokenKind) -> Option<Span> {
        if self.at(kind) {
            Some(self.bump().span)
        } else {
            let found = self.peek();
            self.error(
                found.span,
                format!("expected {}", kind.description()),
                format!("found {}", found.kind.description()),
            );
            None
        }
    }

    fn error(&mut self, span: Span, message: impl Into<String>, label: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message).with_primary(span, label));
    }

    /// The source text covered by `span`.
    fn lexeme(&self, span: Span) -> &str {
        &self.src[span.lo().to_usize()..span.hi().to_usize()]
    }

    // --- Items --------------------------------------------------------------

    fn parse_program(mut self) -> ParseResult {
        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Fn) {
                match self.parse_fn() {
                    Some(f) => items.push(Item::Fn(f)),
                    None => self.synchronize_to_item(),
                }
            } else {
                let found = self.peek();
                self.error(
                    found.span,
                    "expected an item",
                    format!("expected a `fn`, found {}", found.kind.description()),
                );
                self.synchronize_to_item();
            }
        }

        ParseResult {
            program: Program { items },
            diagnostics: self.diagnostics,
        }
    }

    /// Skip tokens until the next item boundary (a `fn` keyword or end of file),
    /// to recover after a malformed item. Always makes progress.
    fn synchronize_to_item(&mut self) {
        while !self.at(TokenKind::Eof) && !self.at(TokenKind::Fn) {
            self.bump();
        }
    }

    /// Parse a function: `fn NAME() -> TYPE { ... }`. Assumes the current token
    /// is `fn`.
    fn parse_fn(&mut self) -> Option<FnDecl> {
        let fn_span = self.bump().span; // `fn`
        let name = self.parse_ident()?;
        self.expect(TokenKind::LParen)?;
        // Parameters are not supported yet: only an empty list is accepted.
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Arrow)?;
        let return_type = self.parse_type()?;
        let body = self.parse_block()?;
        let span = fn_span.to(body.span);
        Some(FnDecl {
            name,
            return_type,
            body,
            span,
        })
    }

    fn parse_ident(&mut self) -> Option<Ident> {
        if self.at(TokenKind::Ident) {
            let token = self.bump();
            Some(Ident {
                name: self.lexeme(token.span).to_string(),
                span: token.span,
            })
        } else {
            let found = self.peek();
            self.error(
                found.span,
                "expected an identifier",
                format!("found {}", found.kind.description()),
            );
            None
        }
    }

    fn parse_type(&mut self) -> Option<Type> {
        Some(Type {
            name: self.parse_ident()?,
        })
    }

    // --- Statements ---------------------------------------------------------

    fn parse_block(&mut self) -> Option<Block> {
        let open = self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.pos;
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            }
            // Guarantee forward progress even when a statement failed to parse.
            if self.pos == before {
                self.bump();
            }
        }
        let close = self
            .expect(TokenKind::RBrace)
            .unwrap_or_else(|| self.peek().span);
        Some(Block {
            stmts,
            span: open.to(close),
        })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        if self.at(TokenKind::Let) {
            self.parse_let_stmt().map(Stmt::Let)
        } else if self.at(TokenKind::Return) {
            self.parse_return_stmt().map(Stmt::Return)
        } else if self.at(TokenKind::If) {
            self.parse_if_stmt().map(Stmt::If)
        } else {
            let found = self.peek();
            self.error(
                found.span,
                "expected a statement",
                format!("found {}", found.kind.description()),
            );
            None
        }
    }

    /// Parse an `if <cond> { … } [else { … } | else if …]`. Assumes the current
    /// token is `if`. The condition is an expression; because expressions never
    /// begin with `{`, the block's opening brace is unambiguous.
    fn parse_if_stmt(&mut self) -> Option<IfStmt> {
        let kw = self.bump().span; // `if`
        let cond = self.parse_expr(0);
        let then_block = self.parse_block()?;
        let mut end = then_block.span;

        let else_branch = if self.at(TokenKind::Else) {
            self.bump(); // `else`
            if self.at(TokenKind::If) {
                // `else if …` — a chained conditional.
                let nested = self.parse_if_stmt()?;
                end = nested.span;
                Some(ElseBranch::If(Box::new(nested)))
            } else {
                let block = self.parse_block()?;
                end = block.span;
                Some(ElseBranch::Block(block))
            }
        } else {
            None
        };

        Some(IfStmt {
            cond,
            then_block,
            else_branch,
            span: kw.to(end),
        })
    }

    fn parse_let_stmt(&mut self) -> Option<LetStmt> {
        let kw = self.bump().span; // `let`
        let name = self.parse_ident()?;
        self.expect(TokenKind::Eq)?;
        let init = self.parse_expr(0);
        let end = self
            .expect(TokenKind::Semicolon)
            .unwrap_or_else(|| init.span());
        Some(LetStmt {
            name,
            span: kw.to(end),
            init,
        })
    }

    fn parse_return_stmt(&mut self) -> Option<ReturnStmt> {
        let kw = self.bump().span; // `return`
        let expr = self.parse_expr(0);
        let end = self
            .expect(TokenKind::Semicolon)
            .unwrap_or_else(|| expr.span());
        Some(ReturnStmt {
            span: kw.to(end),
            expr,
        })
    }

    // --- Expressions (Pratt) ------------------------------------------------

    fn parse_expr(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix();
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Ne,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::Ge,
                TokenKind::AmpAmp => BinOp::And,
                TokenKind::PipePipe => BinOp::Or,
                _ => break,
            };
            let (l_bp, r_bp) = infix_binding_power(op);
            if l_bp < min_bp {
                break;
            }
            self.bump(); // the operator
            let rhs = self.parse_expr(r_bp);
            let span = lhs.span().to(rhs.span());
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        lhs
    }

    fn parse_prefix(&mut self) -> Expr {
        let token = self.peek();
        match token.kind {
            TokenKind::Int => {
                self.bump();
                let value = self.parse_int(token.span);
                Expr::IntLit {
                    value,
                    span: token.span,
                }
            }
            TokenKind::True | TokenKind::False => {
                self.bump();
                Expr::BoolLit {
                    value: token.kind == TokenKind::True,
                    span: token.span,
                }
            }
            TokenKind::Minus => self.parse_prefix_unary(token, UnOp::Neg),
            TokenKind::Bang => self.parse_prefix_unary(token, UnOp::Not),
            TokenKind::LParen => {
                self.bump();
                let inner = self.parse_expr(0);
                self.expect(TokenKind::RParen);
                inner
            }
            TokenKind::Ident => {
                let token = self.bump();
                Expr::Name {
                    name: self.lexeme(token.span).to_string(),
                    span: token.span,
                }
            }
            _ => {
                self.error(
                    token.span,
                    "expected an expression",
                    format!("found {}", token.kind.description()),
                );
                Expr::Error { span: token.span }
            }
        }
    }

    /// Parse a prefix unary operator (`-` or `!`) whose token is `op_token`.
    fn parse_prefix_unary(&mut self, op_token: Token, op: UnOp) -> Expr {
        self.bump(); // the operator
        let operand = self.parse_expr(UNARY_BINDING_POWER);
        let span = op_token.span.to(operand.span());
        Expr::Unary {
            op,
            operand: Box::new(operand),
            span,
        }
    }

    /// Parse the integer value of a literal, reporting overflow.
    fn parse_int(&mut self, span: Span) -> u64 {
        // Own the lexeme so the immutable borrow of `self` ends before `error`
        // needs a mutable one.
        let text = self.lexeme(span).to_string();
        match text.parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                self.error(
                    span,
                    format!("integer literal `{text}` is too large"),
                    "does not fit in a 64-bit integer",
                );
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::SourceMap;

    /// Parse `src` and return the pretty-printed AST plus any diagnostics.
    fn parse_str(src: &str) -> (String, Vec<Diagnostic>) {
        let mut map = SourceMap::new();
        let file = map.add_file("test.ae", src);
        let tokens = aether_lexer::tokenize(map.file(file)).tokens;
        let result = parse(map.file(file), &tokens);
        (
            aether_ast::pretty::print(&result.program),
            result.diagnostics,
        )
    }

    /// Parse `src`, asserting there are no diagnostics, and return the tree.
    fn parse_ok(src: &str) -> String {
        let (tree, diags) = parse_str(src);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        tree
    }

    #[test]
    fn full_minimal_program() {
        let tree = parse_ok("fn main() -> int { return 1 + 2 * 3; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"main\" -> \"int\"
    Block
      Return
        Binary +
          IntLit 1
          Binary *
            IntLit 2
            IntLit 3"
        );
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        // 1 + 2 * 3  =>  1 + (2 * 3)
        let tree = parse_ok("fn f() -> int { return 1 + 2 * 3; }");
        assert!(tree.contains("Binary +"));
        // The `*` node is nested under the `+` node (deeper indentation).
        let plus = tree.find("Binary +").unwrap();
        let star = tree.find("Binary *").unwrap();
        assert!(star > plus, "expected `*` nested under `+`");
    }

    #[test]
    fn subtraction_is_left_associative() {
        // 1 - 2 - 3  =>  (1 - 2) - 3
        let tree = parse_ok("fn f() -> int { return 1 - 2 - 3; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"int\"
    Block
      Return
        Binary -
          Binary -
            IntLit 1
            IntLit 2
          IntLit 3"
        );
    }

    #[test]
    fn unary_minus_binds_tighter_than_multiplication() {
        // -2 * 3  =>  (-2) * 3
        let tree = parse_ok("fn f() -> int { return -2 * 3; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"int\"
    Block
      Return
        Binary *
          Unary -
            IntLit 2
          IntLit 3"
        );
    }

    #[test]
    fn parentheses_override_precedence() {
        // (1 + 2) * 3
        let tree = parse_ok("fn f() -> int { return (1 + 2) * 3; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"int\"
    Block
      Return
        Binary *
          Binary +
            IntLit 1
            IntLit 2
          IntLit 3"
        );
    }

    #[test]
    fn captures_names_and_values() {
        let tree = parse_ok("fn answer() -> int { return 42; }");
        assert!(tree.contains("Fn \"answer\" -> \"int\""));
        assert!(tree.contains("IntLit 42"));
    }

    #[test]
    fn missing_expression_reports_and_recovers() {
        let (tree, diags) = parse_str("fn f() -> int { return ; }");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("expected an expression"));
        // The tree still has the function and a poison node.
        assert!(tree.contains("Return"));
        assert!(tree.contains("Error"));
    }

    #[test]
    fn missing_semicolon_reports() {
        let (_tree, diags) = parse_str("fn f() -> int { return 1 }");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("expected `;`"));
    }

    #[test]
    fn integer_overflow_reports() {
        let (_tree, diags) = parse_str("fn f() -> int { return 99999999999999999999999; }");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("too large"));
    }

    #[test]
    fn recovers_to_next_function_after_error() {
        // The first function is malformed (missing return type); the second is
        // fine. We should still parse the second and report the first's error.
        let (tree, diags) = parse_str("fn bad() -> { } fn good() -> int { return 1; }");
        assert!(!diags.is_empty());
        assert!(tree.contains("Fn \"good\" -> \"int\""));
        assert!(tree.contains("IntLit 1"));
    }

    #[test]
    fn empty_input_is_empty_program() {
        let tree = parse_ok("");
        assert_eq!(tree, "Program");
    }

    #[test]
    fn parses_let_bindings_and_name_references() {
        let tree = parse_ok("fn main() -> int { let x = 1 + 2; return x * x; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"main\" -> \"int\"
    Block
      Let \"x\"
        Binary +
          IntLit 1
          IntLit 2
      Return
        Binary *
          Name \"x\"
          Name \"x\""
        );
    }

    #[test]
    fn multiple_statements_in_order() {
        let tree = parse_ok("fn main() -> int { let a = 1; let b = 2; return a; }");
        // Two `let`s then a `return`, in source order.
        let a = tree.find("Let \"a\"").unwrap();
        let b = tree.find("Let \"b\"").unwrap();
        let ret = tree.find("Return").unwrap();
        assert!(a < b && b < ret, "statements out of order:\n{tree}");
    }

    #[test]
    fn let_missing_equals_reports() {
        let (_tree, diags) = parse_str("fn f() -> int { let x 5; return x; }");
        assert!(diags.iter().any(|d| d.message.contains("expected `=`")));
    }

    #[test]
    fn parses_boolean_literals() {
        let tree = parse_ok("fn f() -> bool { return true; }");
        assert!(tree.contains("BoolLit true"));
        let tree = parse_ok("fn f() -> bool { return false; }");
        assert!(tree.contains("BoolLit false"));
    }

    #[test]
    fn comparison_binds_looser_than_arithmetic() {
        // 1 + 2 < 3 * 4  =>  (1 + 2) < (3 * 4)
        let tree = parse_ok("fn f() -> bool { return 1 + 2 < 3 * 4; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"bool\"
    Block
      Return
        Binary <
          Binary +
            IntLit 1
            IntLit 2
          Binary *
            IntLit 3
            IntLit 4"
        );
    }

    #[test]
    fn equality_binds_looser_than_relational() {
        // a == b < c  =>  a == (b < c)
        let tree = parse_ok("fn f() -> bool { return a == b < c; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"bool\"
    Block
      Return
        Binary ==
          Name \"a\"
          Binary <
            Name \"b\"
            Name \"c\""
        );
    }

    #[test]
    fn logical_not_binds_tighter_than_comparison() {
        // !a == b  =>  (!a) == b
        let tree = parse_ok("fn f() -> bool { return !a == b; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"bool\"
    Block
      Return
        Binary ==
          Unary !
            Name \"a\"
          Name \"b\""
        );
    }

    #[test]
    fn all_comparison_operators_parse() {
        for sym in ["==", "!=", "<", "<=", ">", ">="] {
            let src = format!("fn f() -> bool {{ return 1 {sym} 2; }}");
            let tree = parse_ok(&src);
            assert!(
                tree.contains(&format!("Binary {sym}")),
                "missing `Binary {sym}` in:\n{tree}"
            );
        }
    }

    #[test]
    fn parses_if_else() {
        let tree = parse_ok("fn f() -> int { if x < 0 { return 1; } else { return 2; } }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"int\"
    Block
      If
        Binary <
          Name \"x\"
          IntLit 0
        Then
          Return
            IntLit 1
        Else
          Return
            IntLit 2"
        );
    }

    #[test]
    fn parses_if_without_else() {
        let tree = parse_ok("fn f() -> int { if c { return 1; } return 2; }");
        assert!(tree.contains("If"));
        assert!(tree.contains("Then"));
        assert!(!tree.contains("Else"), "unexpected Else in:\n{tree}");
        // The trailing `return 2;` is a sibling of the `if`, not inside it.
        let then = tree.find("Then").unwrap();
        let ret2 = tree.rfind("Return").unwrap();
        assert!(ret2 > then);
    }

    #[test]
    fn parses_else_if_chain() {
        let tree = parse_ok(
            "fn f() -> int { if a { return 1; } else if b { return 2; } else { return 3; } }",
        );
        // The `else if` nests a second `If` under the first's `Else`.
        assert_eq!(
            tree.matches("If").count(),
            2,
            "expected two If nodes:\n{tree}"
        );
        let first_else = tree.find("Else").unwrap();
        let second_if = tree.rfind("If").unwrap();
        assert!(second_if > first_else, "nested if not under else:\n{tree}");
    }

    #[test]
    fn logical_operator_precedence() {
        // a || b && c  =>  a || (b && c)   (&& binds tighter than ||)
        let tree = parse_ok("fn f() -> bool { return a || b && c; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"bool\"
    Block
      Return
        Binary ||
          Name \"a\"
          Binary &&
            Name \"b\"
            Name \"c\""
        );
    }

    #[test]
    fn logical_and_binds_looser_than_comparison() {
        // a < b && c == d  =>  (a < b) && (c == d)
        let tree = parse_ok("fn f() -> bool { return a < b && c == d; }");
        assert_eq!(
            tree,
            "\
Program
  Fn \"f\" -> \"bool\"
    Block
      Return
        Binary &&
          Binary <
            Name \"a\"
            Name \"b\"
          Binary ==
            Name \"c\"
            Name \"d\""
        );
    }

    #[test]
    fn if_condition_is_not_confused_by_the_block_brace() {
        // `if a < b { … }` — the `<` comparison, then the block, with no ambiguity.
        let tree = parse_ok("fn f() -> bool { if a < b { return true; } return false; }");
        assert!(tree.contains("Binary <"));
    }
}
