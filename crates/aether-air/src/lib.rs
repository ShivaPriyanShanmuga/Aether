//! AIR — the Aether Intermediate Representation.
//!
//! AIR is the reusable core of the compiler platform: the target of AST lowering
//! and the input to every analysis, optimization, and backend. It is a **typed,
//! SSA-based** representation over a **control-flow graph** of basic blocks.
//!
//! # Representation
//!
//! A [`Function`] owns flat arenas (the counterpart to the AST's `Box` tree, see
//! ADR-0011): a value table addressed by [`Value`] and basic blocks addressed by
//! [`Block`]. A [`BlockData`] holds its parameters, an ordered list of the
//! [`Value`]s computed in it, and a [`Terminator`]. Every value is defined either
//! by an instruction (its result, constants included) or as a **block parameter**
//! — see [`ValueDef`]. Block parameters are AIR's SSA merge mechanism (ADR-0017):
//! a predecessor supplies a [`BranchTarget`]'s arguments on the edge it takes, and
//! they become the successor's parameter values.
//!
//! # Facilities
//!
//! - [`print`] renders a module to AIR's textual form (for `--dump-air` and
//!   golden tests).
//! - [`verify`] checks structural invariants and returns any [`VerifyError`]s.
//!
//! AIR is intentionally independent of the frontend: this crate depends only on
//! [`aether_source`]. AST → AIR lowering lives in the separate `aether-lower`
//! crate. See ADR-0013 in `DECISIONS.md` for the ratified design.

mod ir;
mod print;
mod verify;

pub use ir::{
    BinaryOp, Block, BlockData, BranchTarget, CmpOp, Function, InstData, Module, Terminator, Type,
    UnaryOp, Value, ValueDef,
};
pub use print::print;
pub use verify::{VerifyError, verify};
