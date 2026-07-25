//! The AIR interpreter.

use aether_air::{BinaryOp, Function, InstData, Module, Terminator, UnaryOp, Value};
use aether_source::Span;

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
pub fn interpret(module: &Module) -> Result<i64, RunError> {
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
/// particular, that its entry block is terminated and every operand is defined
/// before use.
///
/// # Errors
/// Returns a [`RunError`] if execution fails (e.g. division by zero).
pub fn run_function(function: &Function) -> Result<i64, RunError> {
    // Values are dense (0..value_count) and, within a single block, defined in
    // execution order, so a flat vector indexed by `Value` suffices.
    let mut values = vec![0i64; function.value_count()];
    let block = function.block(function.entry());

    for &value in &block.body {
        let result = eval(function, &values, value)?;
        values[value.index()] = result;
    }

    let terminator = block
        .terminator
        .expect("interpreter requires a verified (terminated) function");
    match terminator {
        Terminator::Ret(value) => Ok(values[value.index()]),
    }
}

/// Evaluate the instruction that defines `value`, given the results computed so
/// far.
fn eval(function: &Function, values: &[i64], value: Value) -> Result<i64, RunError> {
    let inst = function.inst(value);
    match inst.data {
        InstData::IConst(n) => Ok(n),
        InstData::Unary { op, operand } => {
            let x = values[operand.index()];
            Ok(match op {
                UnaryOp::Neg => x.wrapping_neg(),
            })
        }
        InstData::Binary { op, lhs, rhs } => {
            let a = values[lhs.index()];
            let b = values[rhs.index()];
            match op {
                BinaryOp::Add => Ok(a.wrapping_add(b)),
                BinaryOp::Sub => Ok(a.wrapping_sub(b)),
                BinaryOp::Mul => Ok(a.wrapping_mul(b)),
                BinaryOp::Div => {
                    if b == 0 {
                        Err(RunError::DivisionByZero { span: inst.span })
                    } else {
                        // `wrapping_div` also defines `i64::MIN / -1`.
                        Ok(a.wrapping_div(b))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::SourceMap;

    /// Parse, lower, verify, and interpret `src`.
    fn run_str(src: &str) -> Result<i64, RunError> {
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
}
