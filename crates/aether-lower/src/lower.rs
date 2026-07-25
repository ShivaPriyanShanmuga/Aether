//! The AST → AIR lowering pass.

use std::collections::HashMap;

use aether_air::{
    BinaryOp, Block, CmpOp, Function, InstData, Module, Terminator, Type, UnaryOp, Value,
};
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
    let function = Function::new(decl.name.name.clone(), lower_type(&decl.return_type));
    let entry = function.entry();
    let mut lowerer = FnLowerer {
        function,
        scopes: Vec::new(),
        current: entry,
        diagnostics,
    };
    lowerer.lower_fn_body(&decl.body);
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
    /// A stack of lexical scopes; the innermost is last. Name resolution searches
    /// from innermost outward, giving block scoping and shadowing (TD-0027).
    scopes: Vec<HashMap<String, Value>>,
    /// The block instructions are currently appended to. It advances as control
    /// flow is lowered (e.g. to a branch's `then`/`else`/join block).
    current: Block,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl FnLowerer<'_> {
    /// Lower a function body: the whole body is a lexical scope.
    fn lower_fn_body(&mut self, body: &ast::Block) {
        self.push_scope();
        self.lower_stmts(&body.stmts);
        self.pop_scope();
        // If control falls through the body without a `return`, `self.current` is
        // left unterminated; the verifier reports it as a missing terminator
        // (a "missing return", TD-0020) until semantic analysis (M8) does so.
    }

    /// Lower a sequence of statements into the current block, following control
    /// flow. Returns whether control **falls through** (`true`) or every path
    /// diverged via `return` (`false`), in which case later statements are
    /// unreachable and left unlowered.
    fn lower_stmts(&mut self, stmts: &[ast::Stmt]) -> bool {
        for stmt in stmts {
            if !self.lower_stmt(stmt) {
                return false;
            }
        }
        true
    }

    /// Lower one statement. Returns whether control falls through afterwards.
    fn lower_stmt(&mut self, stmt: &ast::Stmt) -> bool {
        match stmt {
            ast::Stmt::Let(let_stmt) => {
                // Lower the initializer *before* binding the name, so the name is
                // not visible in its own initializer (use-before-def is an error).
                // A later binding of the same name in the same scope rebinds it.
                let value = self.lower_expr(self.current, &let_stmt.init);
                self.bind(let_stmt.name.name.clone(), value);
                true
            }
            ast::Stmt::Return(ret) => {
                let value = self.lower_expr(self.current, &ret.expr);
                self.function
                    .set_terminator(self.current, Terminator::Ret(value));
                false // diverges
            }
            ast::Stmt::If(if_stmt) => self.lower_if(if_stmt),
        }
    }

    /// Lower an `if`/`else` into a control-flow graph: a conditional branch from
    /// the current block to a `then` block and an `else`/join block, with each
    /// arm branching to a shared join where control reconverges.
    ///
    /// Returns whether control falls through past the `if`. It does *not* create a
    /// join when both arms diverge (e.g. both `return`), which keeps the CFG free
    /// of unreachable blocks.
    fn lower_if(&mut self, s: &ast::IfStmt) -> bool {
        let cond = self.lower_expr(self.current, &s.cond);
        let pred = self.current;

        // Then branch, in its own scope.
        let then_block = self.function.append_block();
        self.current = then_block;
        self.push_scope();
        let then_falls = self.lower_stmts(&s.then_block.stmts);
        self.pop_scope();
        let then_exit = self.current;

        // Else branch (if present), in its own scope.
        let else_info = s.else_branch.as_ref().map(|else_branch| {
            let else_block = self.function.append_block();
            self.current = else_block;
            self.push_scope();
            let else_falls = self.lower_else(else_branch);
            self.pop_scope();
            (else_block, else_falls, self.current)
        });

        // Control reconverges at a join unless both arms diverge. Without an
        // `else`, the false edge always reaches the continuation, so a join is
        // always needed.
        let need_join = match &else_info {
            None => true,
            Some((_, else_falls, _)) => then_falls || *else_falls,
        };

        if !need_join {
            let (else_block, _, _) =
                else_info.expect("both arms diverging implies an else branch exists");
            self.function.set_terminator(
                pred,
                Terminator::CondBr {
                    cond,
                    then_block,
                    else_block,
                },
            );
            return false;
        }

        let join = self.function.append_block();
        let else_target = match &else_info {
            Some((else_block, _, _)) => *else_block,
            None => join,
        };
        self.function.set_terminator(
            pred,
            Terminator::CondBr {
                cond,
                then_block,
                else_block: else_target,
            },
        );

        // Each arm that falls through branches to the join.
        if then_falls {
            self.function
                .set_terminator(then_exit, Terminator::Br(join));
        }
        if let Some((_, else_falls, else_exit)) = else_info
            && else_falls
        {
            self.function
                .set_terminator(else_exit, Terminator::Br(join));
        }

        self.current = join;
        true
    }

    /// Lower an `else` branch (a block or a chained `else if`).
    fn lower_else(&mut self, else_branch: &ast::ElseBranch) -> bool {
        match else_branch {
            ast::ElseBranch::Block(block) => self.lower_stmts(&block.stmts),
            ast::ElseBranch::If(nested) => self.lower_if(nested),
        }
    }

    /// Enter a new lexical scope.
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Leave the innermost lexical scope.
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Bind `name` to `value` in the innermost scope.
    fn bind(&mut self, name: String, value: Value) {
        self.scopes
            .last_mut()
            .expect("a scope is always active during lowering")
            .insert(name, value);
    }

    /// Resolve `name` to its bound value, searching innermost scope outward.
    fn resolve(&self, name: &str) -> Option<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    /// Lower an expression, appending instructions to `block` and returning the
    /// [`Value`] holding its result.
    fn lower_expr(&mut self, block: Block, expr: &Expr) -> Value {
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
                if let Some(value) = self.resolve(name) {
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
    fn lowers_if_else_both_arms_return() {
        // Both arms diverge, so no join block is emitted.
        assert_eq!(
            lower_str("fn main() -> int { if true { return 1; } else { return 2; } }"),
            "\
fn main() -> int {
block0:
    %0 = bconst true
    condbr %0, block1, block2
block1:
    %1 = iconst 1
    ret %1
block2:
    %2 = iconst 2
    ret %2
}"
        );
    }

    #[test]
    fn lowers_if_without_else_and_falls_through_to_join() {
        // The false edge and the (here non-taken) fall-through reconverge at the
        // join block, which returns a value defined before the `if`.
        assert_eq!(
            lower_str("fn main() -> int { let x = 5; if x < 3 { return 1; } return x; }"),
            "\
fn main() -> int {
block0:
    %0 = iconst 5
    %1 = iconst 3
    %2 = icmp lt %0, %1
    condbr %2, block1, block2
block1:
    %3 = iconst 1
    ret %3
block2:
    ret %0
}"
        );
    }

    #[test]
    fn both_arms_falling_through_reconverge_at_join() {
        // Neither arm returns; both branch to the join, which returns `x` (defined
        // in the entry, so it dominates the join).
        let air = lower_str(
            "fn main() -> int { let x = 1; if true { let a = 2; } else { let b = 3; } return x; }",
        );
        assert!(air.contains("condbr %"), "expected a condbr:\n{air}");
        // Both arms fall through, so each emits an unconditional `br` to the join.
        assert_eq!(
            air.matches("br block").count(),
            2,
            "two `br` to the join:\n{air}"
        );
        // The join returns `x` (%0), defined in the entry, which dominates it.
        assert!(air.contains("ret %0"), "join returns x:\n{air}");
    }

    #[test]
    fn branch_local_binding_is_not_visible_after_the_if() {
        // `y` is bound inside the then-block's scope and is gone at the join.
        let (_air, diags) = lower_checked("fn main() -> int { if true { let y = 1; } return y; }");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("cannot find `y`"));
    }

    #[test]
    fn outer_binding_visible_inside_a_branch() {
        // `x` from the enclosing scope resolves inside the then-block.
        let (_air, diags) =
            lower_checked("fn main() -> int { let x = 7; if true { return x; } return 0; }");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
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
