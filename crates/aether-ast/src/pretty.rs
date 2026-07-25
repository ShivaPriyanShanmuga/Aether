//! A pretty-printer that renders an AST as an indented tree.
//!
//! The output is intended for debugging and for golden tests, and — because AST
//! nodes are self-contained — it needs no source map. Example:
//!
//! ```text
//! Program
//!   Fn "main" -> "int"
//!     Block
//!       Return
//!         Binary +
//!           IntLit 1
//!           Binary *
//!             IntLit 2
//!             IntLit 3
//! ```

use crate::ast::{Block, Expr, Item, Program, Stmt};

/// Render `program` as an indented tree (two spaces per level, no trailing
/// newline).
#[must_use]
pub fn print(program: &Program) -> String {
    let mut printer = Printer {
        lines: Vec::new(),
        depth: 0,
    };
    printer.program(program);
    printer.lines.join("\n")
}

struct Printer {
    lines: Vec<String>,
    depth: usize,
}

impl Printer {
    fn line(&mut self, text: impl AsRef<str>) {
        let indent = "  ".repeat(self.depth);
        self.lines.push(format!("{indent}{}", text.as_ref()));
    }

    fn program(&mut self, program: &Program) {
        self.line("Program");
        self.depth += 1;
        for item in &program.items {
            self.item(item);
        }
        self.depth -= 1;
    }

    fn item(&mut self, item: &Item) {
        match item {
            Item::Fn(f) => {
                self.line(format!(
                    "Fn \"{}\" -> \"{}\"",
                    f.name.name, f.return_type.name.name
                ));
                self.depth += 1;
                self.block(&f.body);
                self.depth -= 1;
            }
        }
    }

    fn block(&mut self, block: &Block) {
        self.line("Block");
        self.depth += 1;
        for stmt in &block.stmts {
            self.stmt(stmt);
        }
        self.depth -= 1;
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Return(r) => {
                self.line("Return");
                self.depth += 1;
                self.expr(&r.expr);
                self.depth -= 1;
            }
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLit { value, .. } => self.line(format!("IntLit {value}")),
            Expr::Unary { op, operand, .. } => {
                self.line(format!("Unary {}", op.symbol()));
                self.depth += 1;
                self.expr(operand);
                self.depth -= 1;
            }
            Expr::Binary { op, lhs, rhs, .. } => {
                self.line(format!("Binary {}", op.symbol()));
                self.depth += 1;
                self.expr(lhs);
                self.expr(rhs);
                self.depth -= 1;
            }
            Expr::Error { .. } => self.line("Error"),
        }
    }
}
