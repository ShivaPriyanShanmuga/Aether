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

use crate::ir::{Function, InstData, Module, Terminator, Value};

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
    // Writing to a String is infallible; the `let _` acknowledges the Result.
    let _ = write!(
        out,
        "fn {}() -> {} {{",
        function.name,
        function.return_type.name()
    );

    for (index, block) in function.blocks().iter().enumerate() {
        let _ = write!(out, "\nblock{index}:");
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

fn inst_text(function: &Function, value: Value) -> String {
    match function.inst(value).data {
        InstData::IConst(n) => format!("iconst {n}"),
        InstData::BConst(b) => format!("bconst {b}"),
        InstData::Unary { op, operand } => format!("{} {}", op.mnemonic(), value_ref(operand)),
        InstData::Binary { op, lhs, rhs } => {
            format!("{} {}, {}", op.mnemonic(), value_ref(lhs), value_ref(rhs))
        }
        InstData::ICmp { op, lhs, rhs } => {
            format!(
                "icmp {} {}, {}",
                op.mnemonic(),
                value_ref(lhs),
                value_ref(rhs)
            )
        }
    }
}

fn terminator_text(terminator: &Terminator) -> String {
    match terminator {
        Terminator::Ret(value) => format!("ret {}", value_ref(*value)),
        Terminator::Br(target) => format!("br {}", block_ref(*target)),
        Terminator::CondBr {
            cond,
            then_block,
            else_block,
        } => format!(
            "condbr {}, {}, {}",
            value_ref(*cond),
            block_ref(*then_block),
            block_ref(*else_block)
        ),
    }
}

fn block_ref(block: crate::ir::Block) -> String {
    format!("block{}", block.index())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, CmpOp, Function, InstData, Terminator, Type, UnaryOp};
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
        let v = f.push_inst(entry, InstData::IConst(42), Type::Int, span);
        let then_b = f.append_block();
        let else_b = f.append_block();
        let join = f.append_block();
        f.set_terminator(
            entry,
            Terminator::CondBr {
                cond,
                then_block: then_b,
                else_block: else_b,
            },
        );
        f.set_terminator(then_b, Terminator::Br(join));
        f.set_terminator(else_b, Terminator::Br(join));
        f.set_terminator(join, Terminator::Ret(v));

        let mut module = Module::new();
        module.add_function(f);

        assert_eq!(
            print(&module),
            "\
fn f() -> int {
block0:
    %0 = bconst true
    %1 = iconst 42
    condbr %0, block1, block2
block1:
    br block3
block2:
    br block3
block3:
    ret %1
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
