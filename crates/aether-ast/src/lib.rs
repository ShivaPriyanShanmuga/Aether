//! The Abstract Syntax Tree (AST) for the Aether language.
//!
//! The AST is the structured output of the parser and the input to AIR lowering.
//! It is a plain owned tree: interior nodes own their children via [`Box`], which
//! is idiomatic, dependency-free, and ergonomic to pattern-match. The heavier
//! index/arena representation is reserved for AIR, where analyses and in-place
//! mutation need it (see ADR-0011).
//!
//! Nodes are **self-contained**: [`Ident`]s store their text and integer literals
//! store their parsed value, so the tree can be inspected and pretty-printed
//! (see [`pretty`]) without consulting the source map. Every node carries a
//! [`Span`](aether_source::Span) back to the source it came from.

mod ast;
pub mod pretty;

pub use ast::{
    BinOp, Block, ElseBranch, Expr, FnDecl, Ident, IfStmt, Item, LetStmt, Program, ReturnStmt,
    Stmt, Type, UnOp,
};
