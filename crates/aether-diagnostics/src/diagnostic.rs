//! The diagnostic data model: [`Severity`], [`Label`], and [`Diagnostic`].

use aether_source::Span;

/// The severity of a diagnostic.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Severity {
    /// A problem that prevents successful compilation.
    Error,
    /// A potential problem that does not prevent compilation.
    Warning,
    /// Informational context, typically attached alongside another diagnostic.
    Note,
    /// A suggestion for how to resolve a problem.
    Help,
}

impl Severity {
    /// The lowercase word used when rendering this severity (e.g. `"error"`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }

    /// Whether this severity is [`Severity::Error`].
    #[must_use]
    pub fn is_error(self) -> bool {
        matches!(self, Severity::Error)
    }
}

/// Whether a label marks the primary site of a diagnostic or a secondary one.
///
/// Primary labels are rendered with `^`, secondary labels with `-`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LabelStyle {
    /// The main location the diagnostic is about.
    Primary,
    /// A related location that provides context.
    Secondary,
}

/// A span annotated with an optional message, pointing into source code.
#[derive(Clone, Debug)]
pub struct Label {
    /// Whether this is the primary or a secondary label.
    pub style: LabelStyle,
    /// The source region this label points at.
    pub span: Span,
    /// An optional message rendered next to the underline.
    pub message: Option<String>,
}

impl Label {
    /// A primary label with a message.
    #[must_use]
    pub fn primary(span: Span, message: impl Into<String>) -> Label {
        Label {
            style: LabelStyle::Primary,
            span,
            message: Some(message.into()),
        }
    }

    /// A secondary (contextual) label with a message.
    #[must_use]
    pub fn secondary(span: Span, message: impl Into<String>) -> Label {
        Label {
            style: LabelStyle::Secondary,
            span,
            message: Some(message.into()),
        }
    }

    /// A primary label with no message — just an underline.
    #[must_use]
    pub fn primary_only(span: Span) -> Label {
        Label {
            style: LabelStyle::Primary,
            span,
            message: None,
        }
    }
}

/// A structured compiler diagnostic.
///
/// Build one with [`Diagnostic::error`] or [`Diagnostic::warning`] and refine it
/// with the `with_*` combinators, each of which consumes and returns the
/// diagnostic for a fluent style:
///
/// ```
/// # use aether_diagnostics::Diagnostic;
/// # use aether_source::{BytePos, SourceMap, Span};
/// # let mut sources = SourceMap::new();
/// # let file = sources.add_file("main.ae", "let x = 5\n");
/// # let span = Span::new(file, BytePos(4), BytePos(5));
/// let diagnostic = Diagnostic::error("unused variable `x`")
///     .with_code("E0100")
///     .with_primary(span, "never read")
///     .with_note("prefix it with `_` to silence this");
/// ```
#[derive(Clone, Debug)]
pub struct Diagnostic {
    /// The severity of the diagnostic.
    pub severity: Severity,
    /// An optional machine-readable error code (e.g. `"E0001"`).
    pub code: Option<String>,
    /// The primary human-readable message.
    pub message: String,
    /// Labeled source spans, primary and secondary.
    pub labels: Vec<Label>,
    /// Free-standing notes, not tied to a span.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Create a diagnostic with the given severity and message.
    #[must_use]
    pub fn new(severity: Severity, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            severity,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Create an [`Severity::Error`] diagnostic.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(Severity::Error, message)
    }

    /// Create a [`Severity::Warning`] diagnostic.
    #[must_use]
    pub fn warning(message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(Severity::Warning, message)
    }

    /// Attach a machine-readable error code.
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Diagnostic {
        self.code = Some(code.into());
        self
    }

    /// Attach an already-built [`Label`].
    #[must_use]
    pub fn with_label(mut self, label: Label) -> Diagnostic {
        self.labels.push(label);
        self
    }

    /// Attach a primary label with a message.
    #[must_use]
    pub fn with_primary(self, span: Span, message: impl Into<String>) -> Diagnostic {
        self.with_label(Label::primary(span, message))
    }

    /// Attach a secondary (contextual) label with a message.
    #[must_use]
    pub fn with_secondary(self, span: Span, message: impl Into<String>) -> Diagnostic {
        self.with_label(Label::secondary(span, message))
    }

    /// Attach a free-standing note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Diagnostic {
        self.notes.push(note.into());
        self
    }

    /// Whether this diagnostic's severity is [`Severity::Error`].
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity.is_error()
    }

    /// The span of the primary label, if any (falling back to the first label).
    #[must_use]
    pub fn primary_span(&self) -> Option<Span> {
        self.labels
            .iter()
            .find(|label| label.style == LabelStyle::Primary)
            .or_else(|| self.labels.first())
            .map(|label| label.span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_source::{BytePos, SourceMap};

    fn span(lo: u32, hi: u32) -> Span {
        // `FileId`s are only minted by a `SourceMap`; every file here is id 0, so
        // spans built by this helper share a file and compare/merge cleanly.
        let mut map = SourceMap::new();
        let file = map.add_file("test.ae", "");
        Span::new(file, BytePos(lo), BytePos(hi))
    }

    #[test]
    fn severity_strings_and_is_error() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert!(Severity::Error.is_error());
        assert!(!Severity::Warning.is_error());
    }

    #[test]
    fn builder_populates_fields() {
        let d = Diagnostic::error("boom")
            .with_code("E0001")
            .with_primary(span(0, 3), "here")
            .with_secondary(span(5, 6), "and here")
            .with_note("a note");

        assert!(d.is_error());
        assert_eq!(d.code.as_deref(), Some("E0001"));
        assert_eq!(d.message, "boom");
        assert_eq!(d.labels.len(), 2);
        assert_eq!(d.notes, vec!["a note".to_string()]);
    }

    #[test]
    fn primary_span_prefers_primary_label() {
        let d = Diagnostic::error("boom")
            .with_secondary(span(5, 6), "secondary")
            .with_primary(span(0, 3), "primary");
        // Even though the secondary label was added first, the primary wins.
        assert_eq!(d.primary_span(), Some(span(0, 3)));
    }

    #[test]
    fn primary_span_falls_back_to_first_label() {
        let d = Diagnostic::warning("hmm").with_secondary(span(5, 6), "only label");
        assert_eq!(d.primary_span(), Some(span(5, 6)));
    }

    #[test]
    fn primary_span_is_none_without_labels() {
        let d = Diagnostic::error("no labels");
        assert_eq!(d.primary_span(), None);
    }
}
