//! The [`DiagnosticHandler`]: collection point for emitted diagnostics.

use crate::diagnostic::Diagnostic;

/// Buffers diagnostics emitted during a compilation and tracks summary counts.
///
/// Phases hold a mutable reference to a handler and call [`emit`](Self::emit) as
/// they discover problems. Buffering (rather than printing immediately) lets the
/// driver decide when and how to render, count errors to decide whether to abort
/// a later phase, and makes diagnostics straightforward to assert on in tests.
#[derive(Debug, Default)]
pub struct DiagnosticHandler {
    diagnostics: Vec<Diagnostic>,
    error_count: usize,
    warning_count: usize,
}

impl DiagnosticHandler {
    /// Create an empty handler.
    #[must_use]
    pub fn new() -> DiagnosticHandler {
        DiagnosticHandler::default()
    }

    /// Record a diagnostic, updating the error/warning counts.
    pub fn emit(&mut self, diagnostic: Diagnostic) {
        match diagnostic.severity {
            crate::Severity::Error => self.error_count += 1,
            crate::Severity::Warning => self.warning_count += 1,
            crate::Severity::Note | crate::Severity::Help => {}
        }
        self.diagnostics.push(diagnostic);
    }

    /// The number of error-severity diagnostics emitted so far.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// The number of warning-severity diagnostics emitted so far.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    /// Whether any error-severity diagnostic has been emitted.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Whether no diagnostics have been emitted at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// All emitted diagnostics, in emission order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Consume the handler, yielding the emitted diagnostics.
    #[must_use]
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Diagnostic;

    #[test]
    fn new_handler_is_empty() {
        let handler = DiagnosticHandler::new();
        assert!(handler.is_empty());
        assert!(!handler.has_errors());
        assert_eq!(handler.error_count(), 0);
        assert_eq!(handler.warning_count(), 0);
    }

    #[test]
    fn emit_updates_counts() {
        let mut handler = DiagnosticHandler::new();
        handler.emit(Diagnostic::error("e1"));
        handler.emit(Diagnostic::warning("w1"));
        handler.emit(Diagnostic::error("e2"));

        assert_eq!(handler.error_count(), 2);
        assert_eq!(handler.warning_count(), 1);
        assert!(handler.has_errors());
        assert_eq!(handler.diagnostics().len(), 3);
    }

    #[test]
    fn notes_and_help_do_not_count_as_errors() {
        let mut handler = DiagnosticHandler::new();
        handler.emit(Diagnostic::new(crate::Severity::Note, "fyi"));
        handler.emit(Diagnostic::new(crate::Severity::Help, "try this"));
        assert!(!handler.has_errors());
        assert_eq!(handler.error_count(), 0);
        assert_eq!(handler.diagnostics().len(), 2);
    }

    #[test]
    fn into_diagnostics_yields_all() {
        let mut handler = DiagnosticHandler::new();
        handler.emit(Diagnostic::error("only"));
        let diagnostics = handler.into_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "only");
    }
}
