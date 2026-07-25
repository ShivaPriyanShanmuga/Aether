//! The AIR verifier: checks structural invariants of a module.
//!
//! The verifier is the IR's safety net. Lowering and, later, optimization passes
//! are expected to produce valid AIR; the verifier catches violations early with
//! a clear message. It currently checks:
//!
//! - every basic block has a terminator;
//! - every operand refers to a value that is defined before its use;
//! - operand and result types agree (all `int` today);
//! - a `ret` returns a value whose type matches the function's return type.
//!
//! Dominance-based def-before-use across multiple blocks will be added with
//! control flow; today a function has a single block, so definition order
//! suffices.

use crate::ir::{Function, InstData, Module, Terminator, Value};

/// A structural problem found by [`verify`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyError {
    /// A human-readable description of the problem, including the function name.
    pub message: String,
}

/// Verify every function in `module`, returning all problems found (empty if the
/// module is well-formed).
#[must_use]
pub fn verify(module: &Module) -> Vec<VerifyError> {
    let mut errors = Vec::new();
    for function in module.functions() {
        verify_function(function, &mut errors);
    }
    errors
}

fn verify_function(function: &Function, errors: &mut Vec<VerifyError>) {
    let error = |errors: &mut Vec<VerifyError>, msg: String| {
        errors.push(VerifyError {
            message: format!("function `{}`: {msg}", function.name),
        });
    };

    for (index, block) in function.blocks().iter().enumerate() {
        // Each value-defining instruction's operands must be defined earlier.
        for &value in &block.body {
            match function.inst(value).data {
                InstData::IConst(_) => {}
                InstData::Unary { operand, .. } => {
                    check_operand(function, value, operand, errors, &error);
                }
                InstData::Binary { lhs, rhs, .. } => {
                    check_operand(function, value, lhs, errors, &error);
                    check_operand(function, value, rhs, errors, &error);
                }
            }
        }

        // Every block must be terminated.
        match &block.terminator {
            None => error(errors, format!("block{index} has no terminator")),
            Some(Terminator::Ret(value)) => {
                if value.index() >= function.value_count() {
                    error(errors, format!("block{index}: `ret` of undefined value"));
                } else if function.value_type(*value) != function.return_type {
                    error(
                        errors,
                        format!(
                            "block{index}: `ret` type {} does not match return type {}",
                            function.value_type(*value).name(),
                            function.return_type.name()
                        ),
                    );
                }
            }
        }
    }
}

/// Check that `operand` (used by `user`) is defined and defined before its use.
fn check_operand(
    function: &Function,
    user: Value,
    operand: Value,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) {
    if operand.index() >= function.value_count() {
        error(errors, format!("{} uses undefined value", value_ref(user)));
    } else if operand.index() >= user.index() {
        // In a single block, definition order equals execution order, so a valid
        // SSA operand always has a smaller index than its user.
        error(
            errors,
            format!(
                "{} uses {} before it is defined",
                value_ref(user),
                value_ref(operand)
            ),
        );
    }
}

fn value_ref(value: Value) -> String {
    format!("%{}", value.index())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, Function, InstData, Module, Terminator, Type};
    use aether_source::{BytePos, SourceMap, Span};

    fn span() -> Span {
        let mut map = SourceMap::new();
        let file = map.add_file("t.ae", "");
        Span::new(file, BytePos(0), BytePos(0))
    }

    fn module_with(f: Function) -> Module {
        let mut m = Module::new();
        m.add_function(f);
        m
    }

    #[test]
    fn valid_function_verifies() {
        let s = span();
        let mut f = Function::new("main", Type::Int);
        let entry = f.entry();
        let a = f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        let b = f.push_inst(entry, InstData::IConst(2), Type::Int, s);
        let sum = f.push_inst(
            entry,
            InstData::Binary {
                op: BinaryOp::Add,
                lhs: a,
                rhs: b,
            },
            Type::Int,
            s,
        );
        f.set_terminator(entry, Terminator::Ret(sum));

        assert!(verify(&module_with(f)).is_empty());
    }

    #[test]
    fn missing_terminator_is_caught() {
        let s = span();
        let mut f = Function::new("main", Type::Int);
        let entry = f.entry();
        f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        // Deliberately no terminator.

        let errors = verify(&module_with(f));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("no terminator"));
    }

    #[test]
    fn ret_of_undefined_value_is_caught() {
        let mut f = Function::new("main", Type::Int);
        let entry = f.entry();
        // Return a value that was never defined.
        f.set_terminator(entry, Terminator::Ret(Value::from_index(0)));

        let errors = verify(&module_with(f));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("undefined value"));
    }

    #[test]
    fn operand_used_before_definition_is_caught() {
        let s = span();
        let mut f = Function::new("main", Type::Int);
        let entry = f.entry();
        let a = f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        // Reference a value index that is not defined.
        let bad = f.push_inst(
            entry,
            InstData::Binary {
                op: BinaryOp::Add,
                lhs: a,
                rhs: Value::from_index(9),
            },
            Type::Int,
            s,
        );
        f.set_terminator(entry, Terminator::Ret(bad));

        let errors = verify(&module_with(f));
        assert!(errors.iter().any(|e| e.message.contains("undefined value")));
    }
}
