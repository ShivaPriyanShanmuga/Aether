//! Human-readable rendering of diagnostics against source code.
//!
//! The output is plain text (no ANSI color) so it is stable to snapshot-test and
//! safe to pipe. Coloring output when stderr is a terminal is a future
//! enhancement tracked in `TECH_DEBT.md`.

use aether_source::SourceMap;

use crate::diagnostic::{Diagnostic, Label, LabelStyle};

/// Render `diagnostic` into a multi-line, source-annotated string.
///
/// The general shape is:
///
/// ```text
/// error[E0001]: unexpected token
///  --> main.ae:2:12
///   |
/// 2 | return 1 + ;
///   |            ^ expected an expression
///   |
///   = note: a further explanation
/// ```
///
/// A diagnostic with no labels renders just its header and any notes.
#[must_use]
pub fn render(diagnostic: &Diagnostic, sources: &SourceMap) -> String {
    let mut out = String::new();

    // --- Header: `severity[code]: message` ---
    match &diagnostic.code {
        Some(code) => out.push_str(&format!(
            "{}[{}]: {}",
            diagnostic.severity.as_str(),
            code,
            diagnostic.message
        )),
        None => out.push_str(&format!(
            "{}: {}",
            diagnostic.severity.as_str(),
            diagnostic.message
        )),
    }

    // Gutter width is driven by the largest line number any label references, so
    // that all the `|` separators line up. Minimum width of 1.
    let gutter = diagnostic
        .labels
        .iter()
        .map(|label| {
            sources
                .file(label.span.file())
                .line_col(label.span.lo())
                .line
        })
        .max()
        .map_or(1, |line| line.to_string().len());
    let blank = format!("{:>gutter$} |", "");

    // --- Location line, taken from the primary label (fallback: first label) ---
    let primary = diagnostic
        .labels
        .iter()
        .find(|label| label.style == LabelStyle::Primary)
        .or_else(|| diagnostic.labels.first());
    if let Some(label) = primary {
        let file = sources.file(label.span.file());
        let lc = file.line_col(label.span.lo());
        out.push_str(&format!(
            "\n{:>gutter$}--> {}:{}:{}",
            "",
            file.name(),
            lc.line,
            lc.col
        ));
    }

    // --- One snippet block per label ---
    if !diagnostic.labels.is_empty() {
        out.push('\n');
        out.push_str(&blank);
        for label in &diagnostic.labels {
            render_label(&mut out, label, sources, gutter, &blank);
        }
    }

    // --- Notes ---
    if !diagnostic.notes.is_empty() {
        if !diagnostic.labels.is_empty() {
            out.push('\n');
            out.push_str(&blank);
        }
        for note in &diagnostic.notes {
            out.push_str(&format!("\n{:>gutter$} = note: {note}", ""));
        }
    }

    out
}

/// Append the source line and underline for a single label.
fn render_label(out: &mut String, label: &Label, sources: &SourceMap, gutter: usize, blank: &str) {
    let file = sources.file(label.span.file());
    let start = file.line_col(label.span.lo());
    let end = file.line_col(label.span.hi());
    let line_index = (start.line - 1) as usize;
    let src_line = file.line_text(line_index);

    // Length of the underline, in characters. A multi-line span is underlined to
    // the end of its first line (fuller multi-line rendering is future work).
    let underline_len = if end.line == start.line {
        end.col.saturating_sub(start.col).max(1)
    } else {
        (src_line.chars().count() as u32)
            .saturating_sub(start.col - 1)
            .max(1)
    };

    // Pad up to the caret using the source's own leading characters, so that a
    // tab in the source is matched by a tab here and the caret stays aligned.
    let pad: String = src_line
        .chars()
        .take((start.col - 1) as usize)
        .map(|c| if c == '\t' { '\t' } else { ' ' })
        .collect();

    let marker = match label.style {
        LabelStyle::Primary => '^',
        LabelStyle::Secondary => '-',
    };
    let carets: String = std::iter::repeat_n(marker, underline_len as usize).collect();

    out.push_str(&format!("\n{:>gutter$} | {src_line}", start.line));
    out.push_str(&format!("\n{blank} {pad}{carets}"));
    if let Some(message) = &label.message {
        out.push_str(&format!(" {message}"));
    }
}
