//! Source management for the Aether compiler platform.
//!
//! This crate owns the mapping between compiler inputs and human-facing
//! positions. It provides:
//!
//! - [`SourceMap`], which owns the set of input [`SourceFile`]s;
//! - [`Span`], a compact, `Copy` byte range within a file that is attached to
//!   tokens, AST nodes, and IR values throughout the compiler;
//! - resolution from a byte [`BytePos`] to a 1-based [`LineCol`] for diagnostics.
//!
//! Positions are stored as **byte offsets**, not line/column pairs, so that a
//! [`Span`] stays small and cheap to copy onto every node. Line and column are
//! computed on demand from a line table precomputed once per file. The internal
//! representation of [`Span`] is private behind accessors so it can be optimized
//! later without affecting callers. See `ARCHITECTURE.md` and ADR-0008 in
//! `DECISIONS.md`.

mod pos;
mod source_map;

pub use pos::{BytePos, FileId, LineCol, Span};
pub use source_map::{SourceFile, SourceMap};
