//! AIR's textual form.
//!
//! The format is a small, human-readable assembly-like syntax used for the
//! `--dump-air` flag and for golden tests:
//!
//! ```text
//! fn main() -> int {
//! block0:
//!     %0 = iconst 1
//!     %1 = iconst 2
//!     %2 = mul %0, %1
//!     ret %2
//! }
//! ```

use std::fmt::Write as _;

use crate::ir::{BranchTarget, Function, InstData, Module, Terminator, Value, ValueDef};

/// Render a whole module to AIR's textual form (no trailing newline).
///
/// Functions are separated by a blank line.
#[must_use]
pub fn print(module: &Module) -> String {
    let mut out = String::new();
    for (i, function) in module.functions().iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        print_function(&mut out, function);
    }
    out
}

fn print_function(out: &mut String, function: &Function) {
    // A function's parameters are its entry block's parameters, shown here in the
    // signature (and therefore suppressed on the entry block header below).
    let params: Vec<String> = function
        .params()
        .iter()
        .map(|&p| format!("{}: {}", value_ref(p), function.value_type(p).name()))
        .collect();
    // Writing to a String is infallible; the `let _` acknowledges the Result.
    let _ = write!(
        out,
        "fn {}({}) -> {} {{",
        function.name,
        params.join(", "),
        function.return_type.name()
    );

    let entry = function.entry().index();
    for (index, block) in function.blocks().iter().enumerate() {
        let header = if index == entry {
            String::new()
        } else {
            block_params(function, block)
        };
        let _ = write!(out, "\nblock{index}{header}:");
        for &value in &block.body {
            let _ = write!(
                out,
                "\n    {} = {}",
                value_ref(value),
                inst_text(function, value)
            );
        }
        if let Some(terminator) = &block.terminator {
            let _ = write!(out, "\n    {}", terminator_text(terminator));
        }
    }

    out.push_str("\n}");
}

fn value_ref(value: Value) -> String {
    format!("%{}", value.index())
}

/// A block's parameter list, e.g. `(%3: int, %4: bool)`, or empty if it has none.
fn block_params(function: &Function, block: &crate::ir::BlockData) -> String {
    if block.params.is_empty() {
        return String::new();
    }
    let params: Vec<String> = block
        .params
        .iter()
        .map(|&p| format!("{}: {}", value_ref(p), function.value_type(p).name()))
        .collect();
    format!("({})", params.join(", "))
}

fn inst_text(function: &Function, value: Value) -> String {
    match function.value_def(value) {
        ValueDef::Inst(InstData::IConst(n)) => format!("iconst {n}"),
        ValueDef::Inst(InstData::BConst(b)) => format!("bconst {b}"),
        ValueDef::Inst(InstData::Unary { op, operand }) => {
            format!("{} {}", op.mnemonic(), value_ref(*operand))
        }
        ValueDef::Inst(InstData::Binary { op, lhs, rhs }) => {
            format!("{} {}, {}", op.mnemonic(), value_ref(*lhs), value_ref(*rhs))
        }
        ValueDef::Inst(InstData::ICmp { op, lhs, rhs }) => {
            format!(
                "icmp {} {}, {}",
                op.mnemonic(),
                value_ref(*lhs),
                value_ref(*rhs)
            )
        }
        ValueDef::Inst(InstData::Call { callee, args }) => {
            let args: Vec<String> = args.iter().map(|&a| value_ref(a)).collect();
            format!("call {}({})", callee, args.join(", "))
        }
        // Block parameters live in the block header, not the body, so this is
        // never reached for a well-formed function.
        ValueDef::Param { .. } => String::from("<param>"),
    }
}

fn terminator_text(terminator: &Terminator) -> String {
    match terminator {
        Terminator::Ret(value) => format!("ret {}", value_ref(*value)),
        Terminator::Br(target) => format!("br {}", target_text(target)),
        Terminator::CondBr {
            cond,
            then_branch,
            else_branch,
        } => format!(
            "condbr {}, {}, {}",
            value_ref(*cond),
            target_text(then_branch),
            target_text(else_branch)
        ),
    }
}

/// A branch target, e.g. `block3` or `block3(%1, %2)` when it passes arguments.
fn target_text(target: &BranchTarget) -> String {
    if target.args.is_empty() {
        format!("block{}", target.block.index())
    } else {
        let args: Vec<String> = target.args.iter().map(|&a| value_ref(a)).collect();
        format!("block{}({})", target.block.index(), args.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, BranchTarget, CmpOp, Function, InstData, Terminator, Type, UnaryOp};
    use aether_source::{BytePos, SourceMap};

    fn dummy_span() -> aether_source::Span {
        let mut map = SourceMap::new();
        let file = map.add_file("t.ae", "");
        aether_source::Span::new(file, BytePos(0), BytePos(0))
    }

    #[test]
    fn prints_boolean_and_comparison_instructions() {
        let span = dummy_span();
        let mut f = Function::new("f", Type::Bool);
        let entry = f.entry();
        let a = f.push_inst(entry, InstData::IConst(1), Type::Int, span);
        let b = f.push_inst(entry, InstData::IConst(2), Type::Int, span);
        let lt = f.push_inst(
            entry,
            InstData::ICmp {
                op: CmpOp::Lt,
                lhs: a,
                rhs: b,
            },
            Type::Bool,
            span,
        );
        let neg = f.push_inst(
            entry,
            InstData::Unary {
                op: UnaryOp::Not,
                operand: lt,
            },
            Type::Bool,
            span,
        );
        f.set_terminator(entry, Terminator::Ret(neg));

        let mut module = Module::new();
        module.add_function(f);

        assert_eq!(
            print(&module),
            "\
fn f() -> bool {
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
    fn prints_branch_terminators_and_multiple_blocks() {
        let span = dummy_span();
        let mut f = Function::new("f", Type::Int);
        let entry = f.entry();
        let cond = f.push_inst(entry, InstData::BConst(true), Type::Bool, span);
        let then_b = f.append_block();
        let else_b = f.append_block();
        let join = f.append_block();
        // The join takes a parameter — the value merged from the two arms.
        let param = f.append_block_param(join, Type::Int, span);
        f.set_terminator(
            entry,
            Terminator::CondBr {
                cond,
                then_branch: BranchTarget::new(then_b),
                else_branch: BranchTarget::new(else_b),
            },
        );
        let ten = f.push_inst(then_b, InstData::IConst(10), Type::Int, span);
        f.set_terminator(
            then_b,
            Terminator::Br(BranchTarget::with_args(join, vec![ten])),
        );
        let twenty = f.push_inst(else_b, InstData::IConst(20), Type::Int, span);
        f.set_terminator(
            else_b,
            Terminator::Br(BranchTarget::with_args(join, vec![twenty])),
        );
        f.set_terminator(join, Terminator::Ret(param));

        let mut module = Module::new();
        module.add_function(f);

        assert_eq!(
            print(&module),
            "\
fn f() -> int {
block0:
    %0 = bconst true
    condbr %0, block1, block2
block1:
    %2 = iconst 10
    br block3(%2)
block2:
    %3 = iconst 20
    br block3(%3)
block3(%1: int):
    ret %1
}"
        );
    }

    #[test]
    fn prints_function_parameters_and_calls() {
        let span = dummy_span();
        // fn add(%0: int, %1: int) -> int { %2 = add %0, %1; ret %2 }
        let mut add = Function::new("add", Type::Int);
        let a = add.append_param(Type::Int, span);
        let b = add.append_param(Type::Int, span);
        let entry = add.entry();
        let sum = add.push_inst(
            entry,
            InstData::Binary {
                op: BinaryOp::Add,
                lhs: a,
                rhs: b,
            },
            Type::Int,
            span,
        );
        add.set_terminator(entry, Terminator::Ret(sum));

        // fn main() -> int { %0 = 2; %1 = 3; %2 = call add(%0, %1); ret %2 }
        let mut main = Function::new("main", Type::Int);
        let m_entry = main.entry();
        let two = main.push_inst(m_entry, InstData::IConst(2), Type::Int, span);
        let three = main.push_inst(m_entry, InstData::IConst(3), Type::Int, span);
        let call = main.push_inst(
            m_entry,
            InstData::Call {
                callee: "add".to_string(),
                args: vec![two, three],
            },
            Type::Int,
            span,
        );
        main.set_terminator(m_entry, Terminator::Ret(call));

        let mut module = Module::new();
        module.add_function(add);
        module.add_function(main);

        assert_eq!(
            print(&module),
            "\
fn add(%0: int, %1: int) -> int {
block0:
    %2 = add %0, %1
    ret %2
}

fn main() -> int {
block0:
    %0 = iconst 2
    %1 = iconst 3
    %2 = call add(%0, %1)
    ret %2
}"
        );
    }

    #[test]
    fn prints_a_small_function() {
        let mut map = SourceMap::new();
        let file = map.add_file("t.ae", "");
        let span = aether_source::Span::new(file, BytePos(0), BytePos(0));

        let mut f = Function::new("main", Type::Int);
        let entry = f.entry();
        let a = f.push_inst(entry, InstData::IConst(1), Type::Int, span);
        let b = f.push_inst(entry, InstData::IConst(2), Type::Int, span);
        let m = f.push_inst(
            entry,
            InstData::Binary {
                op: BinaryOp::Mul,
                lhs: a,
                rhs: b,
            },
            Type::Int,
            span,
        );
        f.set_terminator(entry, Terminator::Ret(m));

        let mut module = Module::new();
        module.add_function(f);

        assert_eq!(
            print(&module),
            "\
fn main() -> int {
block0:
    %0 = iconst 1
    %1 = iconst 2
    %2 = mul %0, %1
    ret %2
}"
        );
    }
}
