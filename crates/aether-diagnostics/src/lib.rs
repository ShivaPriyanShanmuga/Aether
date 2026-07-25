//! Structured diagnostics for the Aether compiler platform.
//!
//! This crate is how every phase reports problems to the user. It deliberately
//! separates three concerns:
//!
//! - **Construction** — a [`Diagnostic`] is built with a fluent, immutable-style
//!   builder ([`Diagnostic::error`], [`Diagnostic::with_primary`], …). It carries
//!   a severity, an optional error code, a message, labeled [`Span`]s, and notes.
//! - **Collection** — phases emit diagnostics into a [`DiagnosticHandler`], which
//!   buffers them and tracks error/warning counts so the driver can decide when
//!   to stop.
//! - **Rendering** — [`render`] turns a diagnostic plus a
//!   [`SourceMap`](aether_source::SourceMap) into human-readable, caret-annotated
//!   text.
//!
//! Keeping these separate means diagnostics can be inspected and tested as data,
//! and the presentation can evolve (color, alternative formats, IDE protocols)
//! without touching the phases that produce them. See ADR-0009 in `DECISIONS.md`.
//!
//! [`Span`]: aether_source::Span

mod diagnostic;
mod handler;
mod render;

pub use diagnostic::{Diagnostic, Label, LabelStyle, Severity};
pub use handler::DiagnosticHandler;
pub use render::render;
