//! The AIR verifier: checks structural invariants of a module.
//!
//! The verifier is the IR's safety net. Lowering and, later, optimization passes
//! are expected to produce valid AIR; the verifier catches violations early with
//! a clear message. It currently checks:
//!
//! - every basic block has a terminator;
//! - every operand refers to a value that is defined before its use;
//! - each instruction's operand and result types are consistent (e.g. `neg`
//!   takes and produces an `int`, `not` a `bool`, a comparison produces a
//!   `bool`);
//! - a `ret` returns a value whose type matches the function's return type.
//!
//! Dominance-based def-before-use across multiple blocks will be added with
//! control flow; today a function has a single block, so definition order
//! suffices.

use crate::ir::{Function, InstData, Module, Terminator, Type, UnaryOp, Value};

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
        // Each value-defining instruction's operands must be defined earlier and
        // have types consistent with the operation.
        for &value in &block.body {
            check_inst(function, value, errors, &error);
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

/// Check one instruction's operands (defined before use) and the type agreement
/// between its operands and its result.
fn check_inst(
    function: &Function,
    value: Value,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) {
    let inst = function.inst(value);
    let result = inst.ty;
    match inst.data {
        InstData::IConst(_) => check_result(result, Type::Int, value, errors, error),
        InstData::BConst(_) => check_result(result, Type::Bool, value, errors, error),
        InstData::Unary { op, operand } => {
            let (operand_ty, result_ty) = match op {
                UnaryOp::Neg => (Type::Int, Type::Int),
                UnaryOp::Not => (Type::Bool, Type::Bool),
            };
            require_operand_type(function, value, operand, operand_ty, errors, error);
            check_result(result, result_ty, value, errors, error);
        }
        InstData::Binary { lhs, rhs, .. } => {
            require_operand_type(function, value, lhs, Type::Int, errors, error);
            require_operand_type(function, value, rhs, Type::Int, errors, error);
            check_result(result, Type::Int, value, errors, error);
        }
        InstData::ICmp { op, lhs, rhs } => {
            if op.is_equality() {
                // `==`/`!=` accept any single type, but both sides must agree.
                let lhs_ty = operand_type(function, value, lhs, errors, error);
                let rhs_ty = operand_type(function, value, rhs, errors, error);
                if let (Some(l), Some(r)) = (lhs_ty, rhs_ty)
                    && l != r
                {
                    error(
                        errors,
                        format!(
                            "{}: `icmp {}` operands have differing types {} and {}",
                            value_ref(value),
                            op.mnemonic(),
                            l.name(),
                            r.name()
                        ),
                    );
                }
            } else {
                // Relational comparisons require integer operands.
                require_operand_type(function, value, lhs, Type::Int, errors, error);
                require_operand_type(function, value, rhs, Type::Int, errors, error);
            }
            check_result(result, Type::Bool, value, errors, error);
        }
    }
}

/// Validate that `operand` (used by `user`) is defined before its use and return
/// its type, or `None` (recording an error) if it is undefined or used too early.
fn operand_type(
    function: &Function,
    user: Value,
    operand: Value,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) -> Option<Type> {
    if operand.index() >= function.value_count() {
        error(errors, format!("{} uses undefined value", value_ref(user)));
        None
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
        None
    } else {
        Some(function.value_type(operand))
    }
}

/// Require `operand` (used by `user`) to have type `expected`, recording an error
/// otherwise. A malformed (undefined/too-early) operand is reported by
/// [`operand_type`] and skips the type check.
fn require_operand_type(
    function: &Function,
    user: Value,
    operand: Value,
    expected: Type,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) {
    if let Some(actual) = operand_type(function, user, operand, errors, error)
        && actual != expected
    {
        error(
            errors,
            format!(
                "{} expects a {} operand but {} has type {}",
                value_ref(user),
                expected.name(),
                value_ref(operand),
                actual.name()
            ),
        );
    }
}

/// Check that an instruction's declared result type matches what the operation
/// produces.
fn check_result(
    actual: Type,
    expected: Type,
    value: Value,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) {
    if actual != expected {
        error(
            errors,
            format!(
                "{} has result type {} but the operation produces {}",
                value_ref(value),
                actual.name(),
                expected.name()
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
    use crate::ir::{BinaryOp, CmpOp, Function, InstData, Module, Terminator, Type, UnaryOp};
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
    fn valid_boolean_function_verifies() {
        // fn f() -> bool { %0 = iconst 1; %1 = iconst 2; %2 = icmp lt %0,%1;
        //                  %3 = not %2; ret %3 }
        let s = span();
        let mut f = Function::new("f", Type::Bool);
        let entry = f.entry();
        let a = f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        let b = f.push_inst(entry, InstData::IConst(2), Type::Int, s);
        let lt = f.push_inst(
            entry,
            InstData::ICmp {
                op: CmpOp::Lt,
                lhs: a,
                rhs: b,
            },
            Type::Bool,
            s,
        );
        let neg = f.push_inst(
            entry,
            InstData::Unary {
                op: UnaryOp::Not,
                operand: lt,
            },
            Type::Bool,
            s,
        );
        f.set_terminator(entry, Terminator::Ret(neg));

        assert!(verify(&module_with(f)).is_empty());
    }

    #[test]
    fn relational_comparison_on_bool_operand_is_caught() {
        // `icmp lt` requires integer operands; feeding it a bool is a type error.
        let s = span();
        let mut f = Function::new("f", Type::Bool);
        let entry = f.entry();
        let t = f.push_inst(entry, InstData::BConst(true), Type::Bool, s);
        let bad = f.push_inst(
            entry,
            InstData::ICmp {
                op: CmpOp::Lt,
                lhs: t,
                rhs: t,
            },
            Type::Bool,
            s,
        );
        f.set_terminator(entry, Terminator::Ret(bad));

        let errors = verify(&module_with(f));
        assert!(
            errors.iter().any(|e| e.message.contains("expects a int")),
            "expected an operand-type error, got: {errors:?}"
        );
    }

    #[test]
    fn equality_on_mismatched_types_is_caught() {
        // `1 == true` compares an int with a bool.
        let s = span();
        let mut f = Function::new("f", Type::Bool);
        let entry = f.entry();
        let i = f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        let t = f.push_inst(entry, InstData::BConst(true), Type::Bool, s);
        let bad = f.push_inst(
            entry,
            InstData::ICmp {
                op: CmpOp::Eq,
                lhs: i,
                rhs: t,
            },
            Type::Bool,
            s,
        );
        f.set_terminator(entry, Terminator::Ret(bad));

        let errors = verify(&module_with(f));
        assert!(
            errors.iter().any(|e| e.message.contains("differing types")),
            "expected a differing-types error, got: {errors:?}"
        );
    }

    #[test]
    fn logical_not_on_int_is_caught() {
        let s = span();
        let mut f = Function::new("f", Type::Bool);
        let entry = f.entry();
        let i = f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        let bad = f.push_inst(
            entry,
            InstData::Unary {
                op: UnaryOp::Not,
                operand: i,
            },
            Type::Bool,
            s,
        );
        f.set_terminator(entry, Terminator::Ret(bad));

        let errors = verify(&module_with(f));
        assert!(
            errors.iter().any(|e| e.message.contains("expects a bool")),
            "expected a bool-operand error, got: {errors:?}"
        );
    }

    #[test]
    fn returning_bool_from_int_function_is_caught() {
        // fn f() -> int { ret (true) } — result type disagrees with return type.
        let s = span();
        let mut f = Function::new("f", Type::Int);
        let entry = f.entry();
        let t = f.push_inst(entry, InstData::BConst(true), Type::Bool, s);
        f.set_terminator(entry, Terminator::Ret(t));

        let errors = verify(&module_with(f));
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("does not match return type")),
            "expected a return-type error, got: {errors:?}"
        );
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
