//! AIR — the Aether Intermediate Representation.
//!
//! AIR is the reusable core of the compiler platform: the target of AST lowering
//! and the input to every analysis, optimization, and backend. It is a **typed,
//! SSA-based** representation over a **control-flow graph** of basic blocks.
//!
//! # Representation
//!
//! A [`Function`] owns flat arenas (the counterpart to the AST's `Box` tree, see
//! ADR-0011): instructions live in a `Vec` addressed by [`Value`], and basic
//! blocks in a `Vec` addressed by [`Block`]. A [`BlockData`] is an ordered list of
//! the [`Value`]s computed in it, followed by a [`Terminator`]. Every value is the
//! result of an instruction — including integer constants — so operands are
//! uniformly just other [`Value`]s (SSA). Control-flow merges (and thus phi
//! nodes / block parameters) are deferred until control flow is introduced.
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
    BinaryOp, Block, BlockData, Function, Inst, InstData, Module, Terminator, Type, UnaryOp, Value,
};
pub use print::print;
pub use verify::{VerifyError, verify};
