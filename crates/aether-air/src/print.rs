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
        InstData::Unary { op, operand } => format!("{} {}", op.mnemonic(), value_ref(operand)),
        InstData::Binary { op, lhs, rhs } => {
            format!("{} {}, {}", op.mnemonic(), value_ref(lhs), value_ref(rhs))
        }
    }
}

fn terminator_text(terminator: &Terminator) -> String {
    match terminator {
        Terminator::Ret(value) => format!("ret {}", value_ref(*value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, Function, InstData, Terminator, Type};
    use aether_source::{BytePos, SourceMap};

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
