//! The AST → AIR lowering pass.

use aether_air::{BinaryOp, Block, Function, InstData, Module, Terminator, Type, UnaryOp, Value};
use aether_ast::{self as ast, Expr, Program};

/// Lower a parsed [`Program`] into an AIR [`Module`].
///
/// Lowering is mechanical and total: it assumes a well-formed AST (the driver
/// only lowers after a clean parse). Structural validity of the result is the
/// job of [`aether_air::verify`].
#[must_use]
pub fn lower(program: &Program) -> Module {
    let mut module = Module::new();
    for item in &program.items {
        match item {
            ast::Item::Fn(decl) => module.add_function(lower_fn(decl)),
        }
    }
    module
}

fn lower_fn(decl: &ast::FnDecl) -> Function {
    let mut function = Function::new(decl.name.name.clone(), lower_type(&decl.return_type));
    let entry = function.entry();

    // The minimal language's only statement is `return`, and the first one
    // terminates the block (any following statements are unreachable). When more
    // statement kinds exist (M6), this becomes a loop that lowers statements until
    // a terminator is produced. A body with no `return` leaves the block
    // unterminated, which the verifier reports.
    if let Some(ast::Stmt::Return(ret)) = decl.body.stmts.first() {
        let value = lower_expr(&mut function, entry, &ret.expr);
        function.set_terminator(entry, Terminator::Ret(value));
    }

    function
}

/// Map an AST type to an AIR type. Only `int` exists; any named type lowers to
/// `int` until the type system (M8) validates type names.
fn lower_type(_ty: &ast::Type) -> Type {
    Type::Int
}

/// Lower an expression, appending instructions to `block` and returning the
/// [`Value`] holding its result.
fn lower_expr(function: &mut Function, block: Block, expr: &Expr) -> Value {
    match expr {
        Expr::IntLit { value, span } => {
            function.push_inst(block, InstData::IConst(*value as i64), Type::Int, *span)
        }
        Expr::Unary { op, operand, span } => {
            let operand = lower_expr(function, block, operand);
            let op = lower_unop(*op);
            function.push_inst(block, InstData::Unary { op, operand }, Type::Int, *span)
        }
        Expr::Binary { op, lhs, rhs, span } => {
            let lhs = lower_expr(function, block, lhs);
            let rhs = lower_expr(function, block, rhs);
            let op = lower_binop(*op);
            function.push_inst(block, InstData::Binary { op, lhs, rhs }, Type::Int, *span)
        }
        // Poison nodes never reach lowering (it runs only after a clean parse);
        // emit a constant so lowering stays total.
        Expr::Error { span } => function.push_inst(block, InstData::IConst(0), Type::Int, *span),
    }
}

fn lower_binop(op: ast::BinOp) -> BinaryOp {
    match op {
        ast::BinOp::Add => BinaryOp::Add,
        ast::BinOp::Sub => BinaryOp::Sub,
        ast::BinOp::Mul => BinaryOp::Mul,
        ast::BinOp::Div => BinaryOp::Div,
    }
}

fn lower_unop(op: ast::UnOp) -> UnaryOp {
    match op {
        ast::UnOp::Neg => UnaryOp::Neg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::SourceMap;

    /// Parse `src` and lower it, returning the printed AIR.
    fn lower_str(src: &str) -> String {
        let mut map = SourceMap::new();
        let file = map.add_file("t.ae", src);
        let tokens = aether_lexer::tokenize(map.file(file)).tokens;
        let program = aether_parser::parse(map.file(file), &tokens).program;
        let module = lower(&program);
        assert!(
            aether_air::verify(&module).is_empty(),
            "lowered module failed verification"
        );
        aether_air::print(&module)
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
    fn lowers_unary_negation() {
        assert_eq!(
            lower_str("fn f() -> int { return -5; }"),
            "\
fn f() -> int {
block0:
    %0 = iconst 5
    %1 = neg %0
    ret %1
}"
        );
    }

    #[test]
    fn lowers_parenthesized_grouping() {
        assert_eq!(
            lower_str("fn f() -> int { return (1 + 2) * 3; }"),
            "\
fn f() -> int {
block0:
    %0 = iconst 1
    %1 = iconst 2
    %2 = add %0, %1
    %3 = iconst 3
    %4 = mul %2, %3
    ret %4
}"
        );
    }

    #[test]
    fn first_return_terminates_the_block() {
        // The second return is unreachable and is not lowered.
        assert_eq!(
            lower_str("fn f() -> int { return 1; return 2; }"),
            "\
fn f() -> int {
block0:
    %0 = iconst 1
    ret %0
}"
        );
    }
}
