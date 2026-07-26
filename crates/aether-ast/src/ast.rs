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

/// A function definition: `fn NAME(PARAMS) -> TYPE { ... }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnDecl {
    /// The function's name.
    pub name: Ident,
    /// The parameters, in declaration order (empty for `()`).
    pub params: Vec<Param>,
    /// The declared return type.
    pub return_type: Type,
    /// The function body.
    pub body: Block,
    /// The span from the `fn` keyword to the closing brace.
    pub span: Span,
}

/// A function parameter: `name: type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    /// The parameter's name.
    pub name: Ident,
    /// The parameter's declared type.
    pub ty: Type,
    /// The span from the name to the type.
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
    /// An `if <cond> { … } [else …]` statement.
    If(IfStmt),
}

impl Stmt {
    /// The source span of this statement.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let(l) => l.span,
            Stmt::Return(r) => r.span,
            Stmt::If(i) => i.span,
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

/// An `if <cond> { … } [else …]` statement.
///
/// `if` is a **statement** (it produces no value); an expression form is a
/// planned addition (see ADR-0019). The optional `else` branch is either another
/// block or a nested `if` (an `else if` chain), captured by [`ElseBranch`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IfStmt {
    /// The condition; expected to be a boolean.
    pub cond: Expr,
    /// The block executed when the condition is true.
    pub then_block: Block,
    /// The optional `else` branch.
    pub else_branch: Option<ElseBranch>,
    /// The span from the `if` keyword to the end of the last branch.
    pub span: Span,
}

/// The `else` branch of an [`IfStmt`]: either a block (`else { … }`) or a nested
/// `if` (`else if …`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElseBranch {
    /// `else { … }`.
    Block(Block),
    /// `else if …` — a chained conditional.
    If(Box<IfStmt>),
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
    /// A boolean literal, `true` or `false`.
    BoolLit {
        /// The literal's value.
        value: bool,
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
    /// A function call, `callee(args)`. The callee is a bare function name (no
    /// first-class functions yet).
    Call {
        /// The called function's name.
        callee: String,
        /// The argument expressions, in order.
        args: Vec<Expr>,
        /// The span from the callee to the closing parenthesis.
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
            | Expr::BoolLit { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Name { span, .. }
            | Expr::Call { span, .. }
            | Expr::Error { span } => *span,
        }
    }
}

/// A binary operator.
///
/// This groups the arithmetic operators (which produce an integer) and the
/// comparison operators (which produce a boolean); they are syntactically
/// uniform infix operators, and lowering distinguishes them by kind.
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
    /// Equality, `==`.
    Eq,
    /// Inequality, `!=`.
    Ne,
    /// Less-than, `<`.
    Lt,
    /// Less-than-or-equal, `<=`.
    Le,
    /// Greater-than, `>`.
    Gt,
    /// Greater-than-or-equal, `>=`.
    Ge,
    /// Short-circuiting logical and, `&&`.
    And,
    /// Short-circuiting logical or, `||`.
    Or,
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
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }

    /// Whether this operator is a comparison (producing a boolean) rather than
    /// an arithmetic operator (producing an integer).
    #[must_use]
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
        )
    }
}

/// A unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    /// Arithmetic negation, `-`.
    Neg,
    /// Logical negation, `!`.
    Not,
}

impl UnOp {
    /// The operator's source symbol (e.g. `"-"`).
    #[must_use]
    pub fn symbol(self) -> &'static str {
        match self {
            UnOp::Neg => "-",
            UnOp::Not => "!",
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
        assert_eq!(BinOp::Eq.symbol(), "==");
        assert_eq!(BinOp::Ne.symbol(), "!=");
        assert_eq!(BinOp::Lt.symbol(), "<");
        assert_eq!(BinOp::Le.symbol(), "<=");
        assert_eq!(BinOp::Gt.symbol(), ">");
        assert_eq!(BinOp::Ge.symbol(), ">=");
        assert_eq!(BinOp::And.symbol(), "&&");
        assert_eq!(BinOp::Or.symbol(), "||");
        assert_eq!(UnOp::Neg.symbol(), "-");
        assert_eq!(UnOp::Not.symbol(), "!");
    }

    #[test]
    fn comparison_classification() {
        assert!(BinOp::Lt.is_comparison());
        assert!(BinOp::Eq.is_comparison());
        assert!(!BinOp::Add.is_comparison());
        assert!(!BinOp::Div.is_comparison());
    }
}
