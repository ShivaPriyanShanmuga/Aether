//! The AIR interpreter.

use std::fmt;

use aether_air::{
    BinaryOp, BranchTarget, CmpOp, Function, InstData, Module, Terminator, UnaryOp, Value, ValueDef,
};
use aether_source::Span;

/// A runtime value produced by executing AIR.
///
/// It mirrors AIR's value types: [`Int`](RunValue::Int) for
/// [`Type::Int`](aether_air::Type::Int) and [`Bool`](RunValue::Bool) for
/// [`Type::Bool`](aether_air::Type::Bool). Verified AIR guarantees each operand
/// carries the variant its operation expects, so the interpreter never has to
/// coerce between them (TD-0024).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunValue {
    /// A 64-bit signed integer.
    Int(i64),
    /// A boolean.
    Bool(bool),
}

impl RunValue {
    /// The integer payload. Panics only if AIR was not type-checked: the verifier
    /// guarantees the operand is an `int` wherever this is called.
    fn as_int(self) -> i64 {
        match self {
            RunValue::Int(n) => n,
            RunValue::Bool(_) => unreachable!("verified AIR guarantees an `int` operand here"),
        }
    }

    /// The boolean payload. Panics only if AIR was not type-checked: the verifier
    /// guarantees the operand is a `bool` wherever this is called.
    fn as_bool(self) -> bool {
        match self {
            RunValue::Bool(b) => b,
            RunValue::Int(_) => unreachable!("verified AIR guarantees a `bool` operand here"),
        }
    }
}

impl fmt::Display for RunValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunValue::Int(n) => write!(f, "{n}"),
            RunValue::Bool(b) => write!(f, "{b}"),
        }
    }
}

/// A failure encountered while executing AIR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// The module has no `main` function to execute.
    NoEntryPoint,
    /// A division (or remainder) had a zero divisor.
    DivisionByZero {
        /// The span of the offending division.
        span: Span,
    },
}

/// Execute a module's `main` function and return its result.
///
/// # Errors
/// Returns [`RunError::NoEntryPoint`] if there is no `main`, or a runtime error
/// such as [`RunError::DivisionByZero`] if execution fails.
pub fn interpret(module: &Module) -> Result<RunValue, RunError> {
    let main = module
        .functions()
        .iter()
        .find(|function| function.name == "main")
        .ok_or(RunError::NoEntryPoint)?;
    run_function(main)
}

/// Execute a single function and return its result.
///
/// Assumes `function` has passed [`aether_air::verify`](aether_air::verify): in
/// particular, that every reachable block is terminated, every operand's
/// definition dominates its use, and operand/result types are consistent.
///
/// Execution walks the control-flow graph: it evaluates the current block's
/// instructions, then follows its terminator to the next block (or returns).
/// Values are dense across the whole function (`0..value_count`), so a single
/// flat vector indexed by `Value` holds all results; dominance guarantees a
/// value is computed before any block that reads it runs. With only `if`/`else`
/// today the CFG is acyclic, so this loop always terminates.
///
/// # Errors
/// Returns a [`RunError`] if execution fails (e.g. division by zero).
pub fn run_function(function: &Function) -> Result<RunValue, RunError> {
    // The initial fill is a placeholder; every slot is written before it is read.
    let mut values = vec![RunValue::Int(0); function.value_count()];
    let mut current = function.entry();
    // Arguments supplied by the edge taken into the current block; empty for the
    // entry (functions have no parameters yet).
    let mut incoming: Vec<RunValue> = Vec::new();

    loop {
        let block = function.block(current);

        // Bind the block's parameters from the arguments the taken edge supplied.
        for (&param, &arg) in block.params.iter().zip(&incoming) {
            values[param.index()] = arg;
        }
        for &value in &block.body {
            values[value.index()] = eval(function, &values, value)?;
        }

        match block
            .terminator
            .as_ref()
            .expect("interpreter requires a verified (terminated) function")
        {
            Terminator::Ret(value) => return Ok(values[value.index()]),
            Terminator::Br(target) => {
                incoming = eval_args(target, &values);
                current = target.block;
            }
            Terminator::CondBr {
                cond,
                then_branch,
                else_branch,
            } => {
                let taken = if values[cond.index()].as_bool() {
                    then_branch
                } else {
                    else_branch
                };
                incoming = eval_args(taken, &values);
                current = taken.block;
            }
        }
    }
}

/// Read the argument values a branch passes to its target's parameters.
fn eval_args(target: &BranchTarget, values: &[RunValue]) -> Vec<RunValue> {
    target.args.iter().map(|&a| values[a.index()]).collect()
}

/// Evaluate the instruction that defines `value`, given the results computed so
/// far.
fn eval(function: &Function, values: &[RunValue], value: Value) -> Result<RunValue, RunError> {
    let data = match function.value_def(value) {
        ValueDef::Inst(data) => *data,
        // Only instruction results are evaluated; parameters are bound from edges.
        ValueDef::Param { .. } => {
            unreachable!("block parameters are bound on entry, not evaluated")
        }
    };
    match data {
        InstData::IConst(n) => Ok(RunValue::Int(n)),
        InstData::BConst(b) => Ok(RunValue::Bool(b)),
        InstData::Unary { op, operand } => {
            let x = values[operand.index()];
            Ok(match op {
                UnaryOp::Neg => RunValue::Int(x.as_int().wrapping_neg()),
                UnaryOp::Not => RunValue::Bool(!x.as_bool()),
            })
        }
        InstData::Binary { op, lhs, rhs } => {
            let a = values[lhs.index()].as_int();
            let b = values[rhs.index()].as_int();
            match op {
                BinaryOp::Add => Ok(RunValue::Int(a.wrapping_add(b))),
                BinaryOp::Sub => Ok(RunValue::Int(a.wrapping_sub(b))),
                BinaryOp::Mul => Ok(RunValue::Int(a.wrapping_mul(b))),
                BinaryOp::Div => {
                    if b == 0 {
                        Err(RunError::DivisionByZero {
                            span: function.value_span(value),
                        })
                    } else {
                        // `wrapping_div` also defines `i64::MIN / -1`.
                        Ok(RunValue::Int(a.wrapping_div(b)))
                    }
                }
            }
        }
        InstData::ICmp { op, lhs, rhs } => {
            let a = values[lhs.index()];
            let b = values[rhs.index()];
            // Equality works over either type (operands are the same type, per
            // verification); relational comparisons operate on integers.
            let result = match op {
                CmpOp::Eq => a == b,
                CmpOp::Ne => a != b,
                CmpOp::Lt => a.as_int() < b.as_int(),
                CmpOp::Le => a.as_int() <= b.as_int(),
                CmpOp::Gt => a.as_int() > b.as_int(),
                CmpOp::Ge => a.as_int() >= b.as_int(),
            };
            Ok(RunValue::Bool(result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::SourceMap;

    /// Parse, lower, verify, and interpret `src`, returning the runtime value.
    fn run_val(src: &str) -> Result<RunValue, RunError> {
        let mut map = SourceMap::new();
        let file = map.add_file("t.ae", src);
        let tokens = aether_lexer::tokenize(map.file(file)).tokens;
        let program = aether_parser::parse(map.file(file), &tokens).program;
        let result = aether_lower::lower(&program);
        assert!(result.diagnostics.is_empty(), "unexpected lowering errors");
        assert!(
            aether_air::verify(&result.module).is_empty(),
            "test module failed verification"
        );
        interpret(&result.module)
    }

    /// Like [`run_val`], but for programs whose `main` returns an `int`.
    fn run_str(src: &str) -> Result<i64, RunError> {
        run_val(src).map(|v| match v {
            RunValue::Int(n) => n,
            RunValue::Bool(b) => panic!("expected an int result, got bool {b}"),
        })
    }

    #[test]
    fn evaluates_arithmetic_with_precedence() {
        assert_eq!(run_str("fn main() -> int { return 1 + 2 * 3; }"), Ok(7));
    }

    #[test]
    fn respects_parentheses() {
        assert_eq!(run_str("fn main() -> int { return (1 + 2) * 3; }"), Ok(9));
    }

    #[test]
    fn evaluates_unary_negation() {
        assert_eq!(run_str("fn main() -> int { return -5 + 8; }"), Ok(3));
    }

    #[test]
    fn subtraction_is_left_associative() {
        assert_eq!(run_str("fn main() -> int { return 10 - 3 - 2; }"), Ok(5));
    }

    #[test]
    fn integer_division_truncates_toward_zero() {
        assert_eq!(run_str("fn main() -> int { return 7 / 2; }"), Ok(3));
    }

    #[test]
    fn division_by_zero_is_a_runtime_error() {
        assert!(matches!(
            run_str("fn main() -> int { return 1 / 0; }"),
            Err(RunError::DivisionByZero { .. })
        ));
    }

    #[test]
    fn missing_main_is_no_entry_point() {
        assert_eq!(
            run_str("fn other() -> int { return 1; }"),
            Err(RunError::NoEntryPoint)
        );
    }

    #[test]
    fn arithmetic_overflow_wraps() {
        // i64::MAX + 1 wraps to i64::MIN.
        let src = format!("fn main() -> int {{ return {} + 1; }}", i64::MAX);
        assert_eq!(run_str(&src), Ok(i64::MIN));
    }

    #[test]
    fn evaluates_local_variables() {
        assert_eq!(
            run_str("fn main() -> int { let x = 1 + 2; return x * x; }"),
            Ok(9)
        );
        assert_eq!(
            run_str("fn main() -> int { let x = 10; let y = x - 3; return x * y; }"),
            Ok(70)
        );
    }

    #[test]
    fn evaluates_boolean_literals() {
        assert_eq!(
            run_val("fn main() -> bool { return true; }"),
            Ok(RunValue::Bool(true))
        );
        assert_eq!(
            run_val("fn main() -> bool { return false; }"),
            Ok(RunValue::Bool(false))
        );
    }

    #[test]
    fn evaluates_relational_comparisons() {
        assert_eq!(
            run_val("fn main() -> bool { return 3 < 5; }"),
            Ok(RunValue::Bool(true))
        );
        assert_eq!(
            run_val("fn main() -> bool { return 5 <= 5; }"),
            Ok(RunValue::Bool(true))
        );
        assert_eq!(
            run_val("fn main() -> bool { return 5 > 7; }"),
            Ok(RunValue::Bool(false))
        );
        assert_eq!(
            run_val("fn main() -> bool { return 2 + 2 >= 5; }"),
            Ok(RunValue::Bool(false))
        );
    }

    #[test]
    fn evaluates_equality_over_both_types() {
        assert_eq!(
            run_val("fn main() -> bool { return 6 * 7 == 42; }"),
            Ok(RunValue::Bool(true))
        );
        assert_eq!(
            run_val("fn main() -> bool { return 1 != 2; }"),
            Ok(RunValue::Bool(true))
        );
        // Equality also works on booleans.
        assert_eq!(
            run_val("fn main() -> bool { return true == true; }"),
            Ok(RunValue::Bool(true))
        );
        assert_eq!(
            run_val("fn main() -> bool { let b = 3 < 5; return b == false; }"),
            Ok(RunValue::Bool(false))
        );
    }

    #[test]
    fn evaluates_logical_not() {
        assert_eq!(
            run_val("fn main() -> bool { return !(1 < 2); }"),
            Ok(RunValue::Bool(false))
        );
        assert_eq!(
            run_val("fn main() -> bool { return !false; }"),
            Ok(RunValue::Bool(true))
        );
    }

    #[test]
    fn boolean_locals_flow_through() {
        // A bool-typed `let` binding is reused like any other SSA value.
        assert_eq!(
            run_val("fn main() -> bool { let ok = 10 >= 10; return ok; }"),
            Ok(RunValue::Bool(true))
        );
    }

    #[test]
    fn if_else_takes_the_true_branch() {
        assert_eq!(
            run_str("fn main() -> int { if 1 < 2 { return 10; } else { return 20; } }"),
            Ok(10)
        );
    }

    #[test]
    fn if_else_takes_the_false_branch() {
        assert_eq!(
            run_str("fn main() -> int { if 2 < 1 { return 10; } else { return 20; } }"),
            Ok(20)
        );
    }

    #[test]
    fn if_without_else_falls_through() {
        // Condition false: skip the then-block, fall through to the trailing return.
        assert_eq!(
            run_str("fn main() -> int { let x = 5; if x < 0 { return 1; } return x; }"),
            Ok(5)
        );
        // Condition true: take the then-block.
        assert_eq!(
            run_str("fn main() -> int { let x = -5; if x < 0 { return 1; } return x; }"),
            Ok(1)
        );
    }

    #[test]
    fn else_if_chain_selects_the_right_arm() {
        let program = |n: i64| {
            format!(
                "fn main() -> int {{ let n = {n}; \
                 if n < 0 {{ return 1; }} \
                 else if n == 0 {{ return 2; }} \
                 else {{ return 3; }} }}"
            )
        };
        assert_eq!(run_str(&program(-4)), Ok(1));
        assert_eq!(run_str(&program(0)), Ok(2));
        assert_eq!(run_str(&program(7)), Ok(3));
    }

    #[test]
    fn value_defined_before_if_is_usable_after() {
        // `x` (defined in the entry) dominates the join and is returned there.
        assert_eq!(
            run_str(
                "fn main() -> int { let x = 42; if true { let y = 1; } else { let z = 2; } return x; }"
            ),
            Ok(42)
        );
    }

    #[test]
    fn logical_and_or_truth_tables() {
        assert_eq!(
            run_val("fn main() -> bool { return true && true; }"),
            Ok(RunValue::Bool(true))
        );
        assert_eq!(
            run_val("fn main() -> bool { return true && false; }"),
            Ok(RunValue::Bool(false))
        );
        assert_eq!(
            run_val("fn main() -> bool { return false || true; }"),
            Ok(RunValue::Bool(true))
        );
        assert_eq!(
            run_val("fn main() -> bool { return false || false; }"),
            Ok(RunValue::Bool(false))
        );
    }

    #[test]
    fn logical_operators_combine_comparisons() {
        assert_eq!(
            run_val("fn main() -> bool { let x = 5; return x > 0 && x < 10; }"),
            Ok(RunValue::Bool(true))
        );
        assert_eq!(
            run_val("fn main() -> bool { let x = 42; return x < 0 || x == 42; }"),
            Ok(RunValue::Bool(true))
        );
    }

    #[test]
    fn and_short_circuits_past_division_by_zero() {
        // `false && …` must not evaluate the right operand, so the `10 / 0` in it
        // is never executed and raises no runtime error.
        assert_eq!(
            run_val("fn main() -> bool { return false && (10 / 0 == 0); }"),
            Ok(RunValue::Bool(false))
        );
    }

    #[test]
    fn or_short_circuits_past_division_by_zero() {
        // `true || …` likewise skips the right operand.
        assert_eq!(
            run_val("fn main() -> bool { return true || (10 / 0 == 0); }"),
            Ok(RunValue::Bool(true))
        );
    }

    #[test]
    fn and_does_evaluate_the_right_operand_when_needed() {
        // When the left operand is true, `&&` evaluates the right — and here that
        // right operand *does* divide by zero, so it is a runtime error.
        assert!(matches!(
            run_val("fn main() -> bool { return true && (10 / 0 == 0); }"),
            Err(RunError::DivisionByZero { .. })
        ));
    }

    #[test]
    fn nested_ifs_execute_correctly() {
        let src = "fn main() -> int { \
             let a = 3; let b = 4; \
             if a < b { \
                 if a == 3 { return 100; } else { return 101; } \
             } else { \
                 return 200; \
             } }";
        assert_eq!(run_str(src), Ok(100));
    }
}
