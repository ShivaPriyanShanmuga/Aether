//! The AST → AIR lowering pass.

use std::collections::HashMap;

use aether_air::{BinaryOp, CmpOp, Function, InstData, Module, Terminator, Type, UnaryOp, Value};
use aether_ast::{self as ast, Expr, Program};
use aether_diagnostics::Diagnostic;
use aether_source::Span;

/// The result of lowering: the AIR module and any diagnostics produced.
///
/// Lowering currently performs a small amount of name resolution (resolving
/// identifiers to the values of `let` bindings), so it can fail — for example on
/// an unknown name. A dedicated name-resolution pass will take over this
/// responsibility later (see ADR-0016).
#[derive(Debug)]
pub struct LowerResult {
    /// The lowered module (best-effort if diagnostics were produced).
    pub module: Module,
    /// Diagnostics emitted during lowering.
    pub diagnostics: Vec<Diagnostic>,
}

/// Lower a parsed [`Program`] into an AIR [`LowerResult`].
///
/// Assumes a well-formed AST (the driver only lowers after a clean parse).
/// Structural validity of the resulting module is checked separately by
/// [`aether_air::verify`].
#[must_use]
pub fn lower(program: &Program) -> LowerResult {
    let mut module = Module::new();
    let mut diagnostics = Vec::new();
    for item in &program.items {
        match item {
            ast::Item::Fn(decl) => module.add_function(lower_fn(decl, &mut diagnostics)),
        }
    }
    LowerResult {
        module,
        diagnostics,
    }
}

fn lower_fn(decl: &ast::FnDecl, diagnostics: &mut Vec<Diagnostic>) -> Function {
    let mut lowerer = FnLowerer {
        function: Function::new(decl.name.name.clone(), lower_type(&decl.return_type)),
        env: HashMap::new(),
        diagnostics,
    };
    lowerer.lower_body(&decl.body.stmts);
    lowerer.function
}

/// Map an AST type name to an AIR type. `int` and `bool` are recognized; any
/// other name falls back to `int` until the type system (M8) validates type
/// names and reports unknown ones (TD-0021).
fn lower_type(ty: &ast::Type) -> Type {
    match ty.name.name.as_str() {
        "bool" => Type::Bool,
        _ => Type::Int,
    }
}

/// Per-function lowering state.
struct FnLowerer<'a> {
    function: Function,
    /// Maps in-scope variable names to the SSA value holding their contents.
    env: HashMap<String, Value>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl FnLowerer<'_> {
    fn lower_body(&mut self, stmts: &[ast::Stmt]) {
        let entry = self.function.entry();
        for stmt in stmts {
            match stmt {
                ast::Stmt::Let(let_stmt) => {
                    // Lower the initializer *before* binding the name, so the name
                    // is not visible in its own initializer (use-before-def is an
                    // error). A later binding of the same name simply rebinds.
                    let value = self.lower_expr(entry, &let_stmt.init);
                    self.env.insert(let_stmt.name.name.clone(), value);
                }
                ast::Stmt::Return(ret) => {
                    let value = self.lower_expr(entry, &ret.expr);
                    self.function.set_terminator(entry, Terminator::Ret(value));
                    // The first `return` terminates the block; any following
                    // statements are unreachable and are left unlowered.
                    break;
                }
            }
        }
    }

    /// Lower an expression, appending instructions to `block` and returning the
    /// [`Value`] holding its result.
    fn lower_expr(&mut self, block: aether_air::Block, expr: &Expr) -> Value {
        match expr {
            Expr::IntLit { value, span } => {
                self.function
                    .push_inst(block, InstData::IConst(*value as i64), Type::Int, *span)
            }
            Expr::BoolLit { value, span } => {
                self.function
                    .push_inst(block, InstData::BConst(*value), Type::Bool, *span)
            }
            Expr::Unary { op, operand, span } => {
                let operand = self.lower_expr(block, operand);
                // The operator determines the result type: `-` on int, `!` on bool.
                let (op, ty) = match op {
                    ast::UnOp::Neg => (UnaryOp::Neg, Type::Int),
                    ast::UnOp::Not => (UnaryOp::Not, Type::Bool),
                };
                self.function
                    .push_inst(block, InstData::Unary { op, operand }, ty, *span)
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let lhs = self.lower_expr(block, lhs);
                let rhs = self.lower_expr(block, rhs);
                // Arithmetic operators produce an int; comparisons produce a bool.
                match lower_binop(*op) {
                    LoweredBinOp::Arith(op) => self.function.push_inst(
                        block,
                        InstData::Binary { op, lhs, rhs },
                        Type::Int,
                        *span,
                    ),
                    LoweredBinOp::Cmp(op) => self.function.push_inst(
                        block,
                        InstData::ICmp { op, lhs, rhs },
                        Type::Bool,
                        *span,
                    ),
                }
            }
            Expr::Name { name, span } => {
                if let Some(&value) = self.env.get(name) {
                    value
                } else {
                    self.error(*span, format!("cannot find `{name}` in this scope"));
                    // Poison value so lowering stays total; the diagnostic stops
                    // the program from being run.
                    self.function
                        .push_inst(block, InstData::IConst(0), Type::Int, *span)
                }
            }
            // Poison nodes never reach lowering (it runs only after a clean parse);
            // emit a constant so lowering stays total.
            Expr::Error { span } => {
                self.function
                    .push_inst(block, InstData::IConst(0), Type::Int, *span)
            }
        }
    }

    fn error(&mut self, span: Span, message: String) {
        self.diagnostics
            .push(Diagnostic::error(message).with_primary(span, "not found in this scope"));
    }
}

/// An AST binary operator lowered to its AIR instruction family: arithmetic
/// (producing an int) or comparison (producing a bool).
enum LoweredBinOp {
    /// An arithmetic operator, lowered to an [`InstData::Binary`].
    Arith(BinaryOp),
    /// A comparison operator, lowered to an [`InstData::ICmp`].
    Cmp(CmpOp),
}

fn lower_binop(op: ast::BinOp) -> LoweredBinOp {
    match op {
        ast::BinOp::Add => LoweredBinOp::Arith(BinaryOp::Add),
        ast::BinOp::Sub => LoweredBinOp::Arith(BinaryOp::Sub),
        ast::BinOp::Mul => LoweredBinOp::Arith(BinaryOp::Mul),
        ast::BinOp::Div => LoweredBinOp::Arith(BinaryOp::Div),
        ast::BinOp::Eq => LoweredBinOp::Cmp(CmpOp::Eq),
        ast::BinOp::Ne => LoweredBinOp::Cmp(CmpOp::Ne),
        ast::BinOp::Lt => LoweredBinOp::Cmp(CmpOp::Lt),
        ast::BinOp::Le => LoweredBinOp::Cmp(CmpOp::Le),
        ast::BinOp::Gt => LoweredBinOp::Cmp(CmpOp::Gt),
        ast::BinOp::Ge => LoweredBinOp::Cmp(CmpOp::Ge),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::SourceMap;

    /// Parse `src` and lower it, returning the printed AIR (asserting no
    /// diagnostics and that the module verifies).
    fn lower_str(src: &str) -> String {
        let (air, diags) = lower_checked(src);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        air
    }

    /// Parse and lower `src`, returning the printed AIR and any diagnostics.
    fn lower_checked(src: &str) -> (String, Vec<Diagnostic>) {
        let mut map = SourceMap::new();
        let file = map.add_file("t.ae", src);
        let tokens = aether_lexer::tokenize(map.file(file)).tokens;
        let program = aether_parser::parse(map.file(file), &tokens).program;
        let LowerResult {
            module,
            diagnostics,
        } = lower(&program);
        (aether_air::print(&module), diagnostics)
    }

    #[test]
    fn lowers_arithmetic_with_precedence() {
        assert_eq!(
            lower_str("fn main() -> int { return 1 + 2 * 3; }"),
            "\
fn main() -> int {
block0:
    %0 = iconst 1
    %1 = iconst 2
    %2 = iconst 3
    %3 = mul %1, %2
    %4 = add %0, %3
    ret %4
}"
        );
    }

    #[test]
    fn lowers_let_bindings_reusing_the_value() {
        // `x` is the `add` result; `x * x` reuses that value (no re-computation).
        assert_eq!(
            lower_str("fn main() -> int { let x = 1 + 2; return x * x; }"),
            "\
fn main() -> int {
block0:
    %0 = iconst 1
    %1 = iconst 2
    %2 = add %0, %1
    %3 = mul %2, %2
    ret %3
}"
        );
    }

    #[test]
    fn later_binding_shadows_earlier() {
        assert_eq!(
            lower_str("fn main() -> int { let x = 1; let x = 2; return x; }"),
            "\
fn main() -> int {
block0:
    %0 = iconst 1
    %1 = iconst 2
    ret %1
}"
        );
    }

    #[test]
    fn lowers_boolean_literal_and_return_type() {
        assert_eq!(
            lower_str("fn main() -> bool { return true; }"),
            "\
fn main() -> bool {
block0:
    %0 = bconst true
    ret %0
}"
        );
    }

    #[test]
    fn lowers_comparison_and_logical_not() {
        assert_eq!(
            lower_str("fn main() -> bool { return !(1 < 2); }"),
            "\
fn main() -> bool {
block0:
    %0 = iconst 1
    %1 = iconst 2
    %2 = icmp lt %0, %1
    %3 = not %2
    ret %3
}"
        );
    }

    #[test]
    fn lowers_each_comparison_operator() {
        for (sym, mnemonic) in [
            ("==", "eq"),
            ("!=", "ne"),
            ("<", "lt"),
            ("<=", "le"),
            (">", "gt"),
            (">=", "ge"),
        ] {
            let air = lower_str(&format!("fn main() -> bool {{ return 1 {sym} 2; }}"));
            assert!(
                air.contains(&format!("icmp {mnemonic} %0, %1")),
                "operator `{sym}` did not lower to `icmp {mnemonic}`:\n{air}"
            );
        }
    }

    #[test]
    fn unknown_name_is_a_diagnostic() {
        let (_air, diags) = lower_checked("fn main() -> int { return y; }");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("cannot find `y`"));
    }

    #[test]
    fn name_not_visible_in_its_own_initializer() {
        // `x` is not in scope inside its own initializer.
        let (_air, diags) = lower_checked("fn main() -> int { let x = x; return x; }");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("cannot find `x`"));
    }
}
