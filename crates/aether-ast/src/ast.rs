//! AST node definitions.

use aether_source::Span;

/// A complete parsed source file: a sequence of top-level items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    /// The top-level items, in source order.
    pub items: Vec<Item>,
}

/// A top-level item.
///
/// Only functions exist today; this is an enum because more item kinds
/// (structs, constants, …) will be added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// A function definition.
    Fn(FnDecl),
}

impl Item {
    /// The source span of this item.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Item::Fn(f) => f.span,
        }
    }
}

/// A function definition: `fn NAME() -> TYPE { ... }`.
///
/// Parameters are not represented yet (the minimal grammar accepts only `()`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnDecl {
    /// The function's name.
    pub name: Ident,
    /// The declared return type.
    pub return_type: Type,
    /// The function body.
    pub body: Block,
    /// The span from the `fn` keyword to the closing brace.
    pub span: Span,
}

/// A type reference. Currently only named types (e.g. `int`) exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type {
    /// The type's name.
    pub name: Ident,
}

impl Type {
    /// The source span of this type (its name).
    #[must_use]
    pub fn span(&self) -> Span {
        self.name.span
    }
}

/// A brace-delimited block of statements: `{ ... }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The statements, in source order.
    pub stmts: Vec<Stmt>,
    /// The span from the opening to the closing brace.
    pub span: Span,
}

/// A statement.
///
/// This is an enum because more statement kinds (expression statements, …) will
/// be added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stmt {
    /// A `let NAME = <expr>;` binding.
    Let(LetStmt),
    /// A `return <expr>;` statement.
    Return(ReturnStmt),
}

impl Stmt {
    /// The source span of this statement.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let(l) => l.span,
            Stmt::Return(r) => r.span,
        }
    }
}

/// A `let NAME = <expr>;` binding of a local variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LetStmt {
    /// The bound name.
    pub name: Ident,
    /// The initializer expression.
    pub init: Expr,
    /// The span from the `let` keyword to the semicolon.
    pub span: Span,
}

/// A `return <expr>;` statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnStmt {
    /// The returned expression.
    pub expr: Expr,
    /// The span from the `return` keyword to the semicolon.
    pub span: Span,
}

/// An expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// An integer literal, with its parsed value.
    IntLit {
        /// The literal's numeric value.
        value: u64,
        /// The literal's span.
        span: Span,
    },
    /// A unary operation, e.g. `-x`.
    Unary {
        /// The operator.
        op: UnOp,
        /// The operand.
        operand: Box<Expr>,
        /// The span covering the operator and operand.
        span: Span,
    },
    /// A binary operation, e.g. `a + b`.
    Binary {
        /// The operator.
        op: BinOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
        /// The span covering both operands.
        span: Span,
    },
    /// A reference to a name, such as a local variable.
    Name {
        /// The referenced name's text.
        name: String,
        /// The reference's span.
        span: Span,
    },
    /// A placeholder produced when parsing failed here, so that recovery can
    /// continue without cascading errors.
    Error {
        /// The span of the offending location.
        span: Span,
    },
}

impl Expr {
    /// The source span of this expression.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Name { span, .. }
            | Expr::Error { span } => *span,
        }
    }
}

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    /// Addition, `+`.
    Add,
    /// Subtraction, `-`.
    Sub,
    /// Multiplication, `*`.
    Mul,
    /// Division, `/`.
    Div,
}

impl BinOp {
    /// The operator's source symbol (e.g. `"+"`).
    #[must_use]
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
        }
    }
}

/// A unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    /// Arithmetic negation, `-`.
    Neg,
}

impl UnOp {
    /// The operator's source symbol (e.g. `"-"`).
    #[must_use]
    pub fn symbol(self) -> &'static str {
        match self {
            UnOp::Neg => "-",
        }
    }
}

/// An identifier: its text and the span it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    /// The identifier's text.
    pub name: String,
    /// The identifier's span.
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::{BytePos, SourceMap};

    fn span(lo: u32, hi: u32) -> Span {
        let mut map = SourceMap::new();
        let file = map.add_file("test.ae", "");
        Span::new(file, BytePos(lo), BytePos(hi))
    }

    #[test]
    fn expr_span_accessor() {
        let e = Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(Expr::IntLit {
                value: 1,
                span: span(0, 1),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 2,
                span: span(4, 5),
            }),
            span: span(0, 5),
        };
        assert_eq!(e.span(), span(0, 5));
    }

    #[test]
    fn operator_symbols() {
        assert_eq!(BinOp::Add.symbol(), "+");
        assert_eq!(BinOp::Sub.symbol(), "-");
        assert_eq!(BinOp::Mul.symbol(), "*");
        assert_eq!(BinOp::Div.symbol(), "/");
        assert_eq!(UnOp::Neg.symbol(), "-");
    }
}
