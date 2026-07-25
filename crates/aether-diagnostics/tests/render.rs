//! Golden tests for diagnostic rendering.
//!
//! These build a real [`SourceMap`] and [`Diagnostic`] and assert on the exact
//! rendered text, locking in caret alignment and layout. Spans are constructed
//! from byte offsets directly (there is no lexer yet).

use aether_diagnostics::{Diagnostic, render};
use aether_source::{BytePos, SourceMap, Span};

/// Build a single-file source map and a span helper over it.
fn setup(name: &str, src: &str) -> (SourceMap, impl Fn(u32, u32) -> Span) {
    let mut sources = SourceMap::new();
    let file = sources.add_file(name, src);
    (sources, move |lo, hi| {
        Span::new(file, BytePos(lo), BytePos(hi))
    })
}

#[test]
fn simple_error_with_primary_label() {
    let (sources, span) = setup("test.ae", "let x = 5\n");
    let diagnostic =
        Diagnostic::error("unused variable `x`").with_primary(span(4, 5), "never read");

    let expected = "\
error: unused variable `x`
 --> test.ae:1:5
  |
1 | let x = 5
  |     ^ never read";
    assert_eq!(render(&diagnostic, &sources), expected);
}

#[test]
fn error_with_code_and_note() {
    let (sources, span) = setup("m.ae", "return 1 + ;\n");
    let diagnostic = Diagnostic::error("unexpected token")
        .with_code("E0001")
        .with_primary(span(11, 12), "expected an expression")
        .with_note("the right-hand side of `+` is missing");

    let expected = "\
error[E0001]: unexpected token
 --> m.ae:1:12
  |
1 | return 1 + ;
  |            ^ expected an expression
  |
  = note: the right-hand side of `+` is missing";
    assert_eq!(render(&diagnostic, &sources), expected);
}

#[test]
fn warning_with_primary_and_secondary_labels() {
    let (sources, span) = setup("shadow.ae", "let x = 1\nlet x = 2\n");
    let diagnostic = Diagnostic::warning("`x` shadows an existing binding")
        .with_secondary(span(4, 5), "previous binding here")
        .with_primary(span(14, 15), "shadows the earlier `x`");

    let expected = "\
warning: `x` shadows an existing binding
 --> shadow.ae:2:5
  |
1 | let x = 1
  |     - previous binding here
2 | let x = 2
  |     ^ shadows the earlier `x`";
    assert_eq!(render(&diagnostic, &sources), expected);
}

#[test]
fn diagnostic_without_labels_renders_header_and_notes() {
    let (sources, _span) = setup("unused.ae", "");
    let diagnostic = Diagnostic::error("the Aether compilation pipeline is not yet implemented")
        .with_note("frontend stages land in Phase 1; see ROADMAP.md");

    let expected = "\
error: the Aether compilation pipeline is not yet implemented
  = note: frontend stages land in Phase 1; see ROADMAP.md";
    assert_eq!(render(&diagnostic, &sources), expected);
}

#[test]
fn multi_character_underline_matches_span_length() {
    let (sources, span) = setup("t.ae", "let name = 5\n");
    // "name" spans bytes 4..8 — four characters, four carets.
    let diagnostic = Diagnostic::error("bad name").with_primary(span(4, 8), "here");

    let expected = "\
error: bad name
 --> t.ae:1:5
  |
1 | let name = 5
  |     ^^^^ here";
    assert_eq!(render(&diagnostic, &sources), expected);
}

#[test]
fn tabs_in_source_are_preserved_in_the_caret_line() {
    // A leading tab must be echoed as a tab in the pad so the caret stays aligned
    // under the source regardless of the terminal's tab width.
    let (sources, span) = setup("tab.ae", "\tx = 5\n");
    let diagnostic = Diagnostic::error("tabbed").with_primary(span(1, 2), "here");

    let expected = "error: tabbed\n --> tab.ae:1:2\n  |\n1 | \tx = 5\n  | \t^ here";
    assert_eq!(render(&diagnostic, &sources), expected);
}

#[test]
fn utf8_columns_align_the_caret() {
    // "héllo" — 'é' is two bytes; the caret should land on the 'l' at column 3.
    let (sources, span) = setup("u.ae", "héllo\n");
    // 'l' is byte offset 3 (h=0, é=1..3, l=3).
    let diagnostic = Diagnostic::error("emphasis").with_primary(span(3, 4), "this one");

    let expected = "\
error: emphasis
 --> u.ae:1:3
  |
1 | héllo
  |   ^ this one";
    assert_eq!(render(&diagnostic, &sources), expected);
}
