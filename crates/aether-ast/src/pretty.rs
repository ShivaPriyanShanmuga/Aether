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

use crate::ast::{Block, ElseBranch, Expr, IfStmt, Item, Program, Stmt};

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
                for param in &f.params {
                    self.line(format!(
                        "Param \"{}\" \"{}\"",
                        param.name.name, param.ty.name.name
                    ));
                }
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
            Stmt::Let(l) => {
                self.line(format!("Let \"{}\"", l.name.name));
                self.depth += 1;
                self.expr(&l.init);
                self.depth -= 1;
            }
            Stmt::Return(r) => {
                self.line("Return");
                self.depth += 1;
                self.expr(&r.expr);
                self.depth -= 1;
            }
            Stmt::If(i) => self.if_stmt(i),
        }
    }

    fn if_stmt(&mut self, i: &IfStmt) {
        self.line("If");
        self.depth += 1;
        self.expr(&i.cond);
        self.line("Then");
        self.depth += 1;
        for stmt in &i.then_block.stmts {
            self.stmt(stmt);
        }
        self.depth -= 1;
        if let Some(else_branch) = &i.else_branch {
            self.line("Else");
            self.depth += 1;
            match else_branch {
                ElseBranch::Block(b) => {
                    for stmt in &b.stmts {
                        self.stmt(stmt);
                    }
                }
                ElseBranch::If(nested) => self.if_stmt(nested),
            }
            self.depth -= 1;
        }
        self.depth -= 1;
    }

    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLit { value, .. } => self.line(format!("IntLit {value}")),
            Expr::BoolLit { value, .. } => self.line(format!("BoolLit {value}")),
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
            Expr::Name { name, .. } => self.line(format!("Name \"{name}\"")),
            Expr::Call { callee, args, .. } => {
                self.line(format!("Call \"{callee}\""));
                self.depth += 1;
                for arg in args {
                    self.expr(arg);
                }
                self.depth -= 1;
            }
            Expr::Error { .. } => self.line("Error"),
        }
    }
}
