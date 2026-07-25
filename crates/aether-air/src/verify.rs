//! The AIR verifier: checks structural invariants of a module.
//!
//! The verifier is the IR's safety net. Lowering and, later, optimization passes
//! are expected to produce valid AIR; the verifier catches violations early with
//! a clear message. It currently checks:
//!
//! - every reachable basic block has a terminator, and every branch targets an
//!   existing block;
//! - every operand refers to a value whose definition **dominates** the use
//!   (i.e. is defined on every path from entry to that use);
//! - each instruction's operand and result types are consistent (e.g. `neg`
//!   takes and produces an `int`, `not` a `bool`, a comparison produces a
//!   `bool`), and a `condbr` condition is a `bool`;
//! - a `ret` returns a value whose type matches the function's return type.
//!
//! Dominance is computed as a forward "availability" dataflow: a value is
//! available at a block iff it is defined on all paths reaching it (the
//! intersection, over predecessors, of the values they make available). This is
//! exactly "the definition dominates the use" for SSA values, and it generalizes
//! the old single-block definition-order check to arbitrary control-flow graphs.
//! Only reachable blocks are verified. Block parameters (ADR-0017) are not yet
//! implemented, so every value is still an instruction result.

use std::collections::{HashMap, HashSet};

use crate::ir::{Block, Function, InstData, Module, Terminator, Type, UnaryOp, Value};

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

    let block_count = function.blocks().len();
    let reachable = reachable_blocks(function);
    let avail_in = compute_availability(function, &reachable);

    for (index, block) in function.blocks().iter().enumerate() {
        let bid = Block::from_index(index);
        // Only reachable blocks are verified; unreachable blocks cannot affect
        // execution and would produce spurious dominance errors.
        if !reachable.contains(&bid) {
            continue;
        }

        // Walk the block, growing the set of values defined at each point: the
        // values available on entry (its dominators' definitions) plus those the
        // block has defined so far.
        let mut defined = avail_in.get(&bid).cloned().unwrap_or_default();
        for &value in &block.body {
            check_inst(function, value, &defined, errors, &error);
            defined.insert(value);
        }

        match &block.terminator {
            None => error(errors, format!("block{index} has no terminator")),
            Some(terminator) => {
                check_terminator(
                    function,
                    index,
                    terminator,
                    &defined,
                    block_count,
                    errors,
                    &error,
                );
            }
        }
    }
}

/// The set of blocks reachable from the entry by following terminators.
fn reachable_blocks(function: &Function) -> HashSet<Block> {
    let block_count = function.blocks().len();
    let mut seen = HashSet::new();
    let mut stack = vec![function.entry()];
    while let Some(block) = stack.pop() {
        if !seen.insert(block) {
            continue;
        }
        if let Some(terminator) = &function.block(block).terminator {
            for succ in terminator.successors() {
                // Ignore out-of-range targets here; `check_terminator` reports them.
                if succ.index() < block_count && !seen.contains(&succ) {
                    stack.push(succ);
                }
            }
        }
    }
    seen
}

/// Compute, for each reachable block, the set of values guaranteed to be defined
/// on entry — i.e. defined on *every* path from the entry. This is the SSA
/// dominance relation for value definitions, obtained by a forward
/// intersection-over-predecessors dataflow run to a fixpoint.
fn compute_availability(
    function: &Function,
    reachable: &HashSet<Block>,
) -> HashMap<Block, HashSet<Value>> {
    let block_count = function.blocks().len();
    let entry = function.entry();

    // Predecessors, restricted to reachable blocks and in-range targets.
    let mut preds: HashMap<Block, Vec<Block>> = HashMap::new();
    // Values defined within each block's body.
    let mut defs: HashMap<Block, HashSet<Value>> = HashMap::new();
    for &block in reachable {
        defs.insert(block, function.block(block).body.iter().copied().collect());
        if let Some(terminator) = &function.block(block).terminator {
            for succ in terminator.successors() {
                if succ.index() < block_count && reachable.contains(&succ) {
                    preds.entry(succ).or_default().push(block);
                }
            }
        }
    }

    // Initialize: entry has nothing available; every other block starts at the
    // universe of all values so that intersection can only shrink it.
    let universe: HashSet<Value> = (0..function.value_count()).map(Value::from_index).collect();
    let mut avail_in: HashMap<Block, HashSet<Value>> = HashMap::new();
    for &block in reachable {
        if block == entry {
            avail_in.insert(block, HashSet::new());
        } else {
            avail_in.insert(block, universe.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for &block in reachable {
            if block == entry {
                continue;
            }
            let new_in = match preds.get(&block) {
                // A reachable non-entry block always has a reachable predecessor.
                None => HashSet::new(),
                Some(block_preds) => {
                    let mut acc: Option<HashSet<Value>> = None;
                    for &p in block_preds {
                        let mut out = avail_in[&p].clone();
                        out.extend(defs[&p].iter().copied());
                        acc = Some(match acc {
                            None => out,
                            Some(current) => current.intersection(&out).copied().collect(),
                        });
                    }
                    acc.unwrap_or_default()
                }
            };
            if new_in != avail_in[&block] {
                avail_in.insert(block, new_in);
                changed = true;
            }
        }
    }

    avail_in
}

/// Check a block's terminator: its operands are defined and correctly typed, and
/// its branch targets exist.
fn check_terminator(
    function: &Function,
    block_index: usize,
    terminator: &Terminator,
    defined: &HashSet<Value>,
    block_count: usize,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) {
    match terminator {
        Terminator::Ret(value) => {
            if let Some(actual) = terminator_operand_type(
                function,
                *value,
                defined,
                errors,
                error,
                block_index,
                "ret",
            ) && actual != function.return_type
            {
                error(
                    errors,
                    format!(
                        "block{block_index}: `ret` type {} does not match return type {}",
                        actual.name(),
                        function.return_type.name()
                    ),
                );
            }
        }
        Terminator::Br(target) => {
            check_block_target(*target, block_count, block_index, errors, error)
        }
        Terminator::CondBr {
            cond,
            then_block,
            else_block,
        } => {
            if let Some(actual) = terminator_operand_type(
                function,
                *cond,
                defined,
                errors,
                error,
                block_index,
                "condbr",
            ) && actual != Type::Bool
            {
                error(
                    errors,
                    format!(
                        "block{block_index}: `condbr` condition has type {} but must be bool",
                        actual.name()
                    ),
                );
            }
            check_block_target(*then_block, block_count, block_index, errors, error);
            check_block_target(*else_block, block_count, block_index, errors, error);
        }
    }
}

/// Validate a branch target refers to an existing block.
fn check_block_target(
    target: Block,
    block_count: usize,
    from_block: usize,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) {
    if target.index() >= block_count {
        error(
            errors,
            format!(
                "block{from_block}: branch to nonexistent block{}",
                target.index()
            ),
        );
    }
}

/// Validate a terminator operand (`ret`/`condbr`) is defined and return its type,
/// or `None` (recording an error) if it is undefined or does not dominate the use.
fn terminator_operand_type(
    function: &Function,
    operand: Value,
    defined: &HashSet<Value>,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
    block_index: usize,
    mnemonic: &str,
) -> Option<Type> {
    if operand.index() >= function.value_count() {
        error(
            errors,
            format!("block{block_index}: `{mnemonic}` of undefined value"),
        );
        None
    } else if !defined.contains(&operand) {
        error(
            errors,
            format!(
                "block{block_index}: `{mnemonic}` uses {} whose definition does not dominate this use",
                value_ref(operand)
            ),
        );
        None
    } else {
        Some(function.value_type(operand))
    }
}

/// Check one instruction's operands (each dominated by its definition) and the
/// type agreement between its operands and its result. `defined` is the set of
/// values available at this instruction.
fn check_inst(
    function: &Function,
    value: Value,
    defined: &HashSet<Value>,
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
            require_operand_type(function, value, operand, operand_ty, defined, errors, error);
            check_result(result, result_ty, value, errors, error);
        }
        InstData::Binary { lhs, rhs, .. } => {
            require_operand_type(function, value, lhs, Type::Int, defined, errors, error);
            require_operand_type(function, value, rhs, Type::Int, defined, errors, error);
            check_result(result, Type::Int, value, errors, error);
        }
        InstData::ICmp { op, lhs, rhs } => {
            if op.is_equality() {
                // `==`/`!=` accept any single type, but both sides must agree.
                let lhs_ty = operand_type(function, value, lhs, defined, errors, error);
                let rhs_ty = operand_type(function, value, rhs, defined, errors, error);
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
                require_operand_type(function, value, lhs, Type::Int, defined, errors, error);
                require_operand_type(function, value, rhs, Type::Int, defined, errors, error);
            }
            check_result(result, Type::Bool, value, errors, error);
        }
    }
}

/// Validate that `operand` (used by `user`) is defined and that its definition
/// dominates the use, returning its type — or `None` (recording an error) if it
/// is undefined or not available on all paths to the use.
fn operand_type(
    function: &Function,
    user: Value,
    operand: Value,
    defined: &HashSet<Value>,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) -> Option<Type> {
    if operand.index() >= function.value_count() {
        error(errors, format!("{} uses undefined value", value_ref(user)));
        None
    } else if !defined.contains(&operand) {
        error(
            errors,
            format!(
                "{} uses {} whose definition does not dominate this use",
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
/// otherwise. A malformed (undefined/non-dominating) operand is reported by
/// [`operand_type`] and skips the type check.
fn require_operand_type(
    function: &Function,
    user: Value,
    operand: Value,
    expected: Type,
    defined: &HashSet<Value>,
    errors: &mut Vec<VerifyError>,
    error: &impl Fn(&mut Vec<VerifyError>, String),
) {
    if let Some(actual) = operand_type(function, user, operand, defined, errors, error)
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
    fn valid_branching_function_verifies() {
        // fn f() -> int { %0=true; %1=42; condbr %0, then, else;
        //                 then: br join; else: br join; join: ret %1 }
        let s = span();
        let mut f = Function::new("f", Type::Int);
        let entry = f.entry();
        let cond = f.push_inst(entry, InstData::BConst(true), Type::Bool, s);
        let v = f.push_inst(entry, InstData::IConst(42), Type::Int, s);
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
        // `%1` is defined in the entry, which dominates the join — a legal use.
        f.set_terminator(join, Terminator::Ret(v));

        assert!(verify(&module_with(f)).is_empty());
    }

    #[test]
    fn use_of_value_not_dominating_is_caught() {
        // A value defined only in the `then` block is used in the join, which the
        // `then` block does not dominate (the false edge skips it).
        let s = span();
        let mut f = Function::new("f", Type::Int);
        let entry = f.entry();
        let cond = f.push_inst(entry, InstData::BConst(true), Type::Bool, s);
        let then_b = f.append_block();
        let join = f.append_block();
        f.set_terminator(
            entry,
            Terminator::CondBr {
                cond,
                then_block: then_b,
                else_block: join,
            },
        );
        let only_in_then = f.push_inst(then_b, InstData::IConst(7), Type::Int, s);
        f.set_terminator(then_b, Terminator::Br(join));
        f.set_terminator(join, Terminator::Ret(only_in_then));

        let errors = verify(&module_with(f));
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("does not dominate")),
            "expected a dominance error, got: {errors:?}"
        );
    }

    #[test]
    fn condbr_on_non_bool_condition_is_caught() {
        let s = span();
        let mut f = Function::new("f", Type::Int);
        let entry = f.entry();
        let n = f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        let then_b = f.append_block();
        let else_b = f.append_block();
        f.set_terminator(
            entry,
            Terminator::CondBr {
                cond: n, // an int, not a bool
                then_block: then_b,
                else_block: else_b,
            },
        );
        f.set_terminator(then_b, Terminator::Ret(n));
        f.set_terminator(else_b, Terminator::Ret(n));

        let errors = verify(&module_with(f));
        assert!(
            errors.iter().any(|e| e.message.contains("must be bool")),
            "expected a condition-type error, got: {errors:?}"
        );
    }

    #[test]
    fn branch_to_nonexistent_block_is_caught() {
        let s = span();
        let mut f = Function::new("f", Type::Int);
        let entry = f.entry();
        let v = f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        // Branch to a block index that does not exist.
        f.set_terminator(entry, Terminator::Br(Block::from_index(9)));
        let _ = v;

        let errors = verify(&module_with(f));
        assert!(
            errors.iter().any(|e| e.message.contains("nonexistent")),
            "expected a bad-target error, got: {errors:?}"
        );
    }

    #[test]
    fn unreachable_block_is_not_verified() {
        // A dead block that is never branched to is ignored (its missing
        // terminator and undominated uses do not matter).
        let s = span();
        let mut f = Function::new("f", Type::Int);
        let entry = f.entry();
        let v = f.push_inst(entry, InstData::IConst(1), Type::Int, s);
        f.set_terminator(entry, Terminator::Ret(v));
        // Append an unreachable, unterminated block.
        let _dead = f.append_block();

        assert!(verify(&module_with(f)).is_empty());
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
