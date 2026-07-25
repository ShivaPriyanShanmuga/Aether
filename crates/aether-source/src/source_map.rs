//! The [`SourceMap`] and [`SourceFile`] types: ownership of input files and
//! resolution of byte positions to line/column locations.

use std::fmt;

use crate::pos::{BytePos, FileId, LineCol, Span};

/// A single source file: its name, contents, and a precomputed line table.
///
/// The line table (`line_starts`) records the byte offset at which each line
/// begins, enabling `O(log n)` resolution of a byte position to a line via
/// binary search. It is computed once, when the file is added to a
/// [`SourceMap`].
pub struct SourceFile {
    id: FileId,
    name: String,
    src: String,
    /// Byte offset of the start of each line. Always begins with `BytePos(0)`;
    /// an entry is pushed immediately after every `\n`.
    line_starts: Vec<BytePos>,
}

impl SourceFile {
    /// This file's identifier within its owning [`SourceMap`].
    #[must_use]
    pub fn id(&self) -> FileId {
        self.id
    }

    /// The file's name (typically a path, or a placeholder like `<anon>`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The full source text.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.src
    }

    /// The length of the file in bytes.
    #[must_use]
    pub fn len_bytes(&self) -> u32 {
        self.src.len() as u32
    }

    /// The number of lines in the file.
    ///
    /// A file ending in a newline has a final, empty line; this count includes
    /// it. An empty file counts as one (empty) line.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// The 0-based index of the line containing `pos`.
    fn line_index(&self, pos: BytePos) -> usize {
        match self.line_starts.binary_search(&pos) {
            // `pos` is exactly a line start.
            Ok(index) => index,
            // `pos` falls within the line whose start precedes the insertion
            // point. `binary_search` never returns 0 here because
            // `line_starts[0]` is `BytePos(0)` and positions are non-negative.
            Err(insertion) => insertion - 1,
        }
    }

    /// The half-open byte range of the given 0-based line, excluding the
    /// trailing `\n` (a trailing `\r` is retained here and trimmed by
    /// [`SourceFile::line_text`]).
    fn line_byte_range(&self, line_index: usize) -> (BytePos, BytePos) {
        let lo = self.line_starts[line_index];
        let hi = if line_index + 1 < self.line_starts.len() {
            // Drop the `\n` that begins the following line.
            BytePos(self.line_starts[line_index + 1].0 - 1)
        } else {
            BytePos(self.src.len() as u32)
        };
        (lo, hi)
    }

    /// The text of the given 0-based line, without its line terminator.
    ///
    /// Both `\n` and a `\r\n` sequence's `\r` are excluded, so callers get the
    /// visible line content regardless of the file's line-ending convention.
    #[must_use]
    pub fn line_text(&self, line_index: usize) -> &str {
        let (lo, hi) = self.line_byte_range(line_index);
        let text = &self.src[lo.to_usize()..hi.to_usize()];
        text.strip_suffix('\r').unwrap_or(text)
    }

    /// Resolve a byte position to a 1-based [`LineCol`].
    ///
    /// The column counts characters from the start of the line. `pos` is clamped
    /// to the end of the file and floored to the nearest character boundary, so
    /// this never panics.
    #[must_use]
    pub fn line_col(&self, pos: BytePos) -> LineCol {
        let line_index = self.line_index(pos);
        let line_start = self.line_starts[line_index].to_usize();

        let mut end = pos.to_usize().min(self.src.len());
        while end > line_start && !self.src.is_char_boundary(end) {
            end -= 1;
        }

        let col = self.src[line_start..end].chars().count() as u32 + 1;
        LineCol {
            line: line_index as u32 + 1,
            col,
        }
    }
}

impl fmt::Debug for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deliberately omit the full source text to keep debug output readable.
        f.debug_struct("SourceFile")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("len_bytes", &self.src.len())
            .field("lines", &self.line_starts.len())
            .finish()
    }
}

/// Owns the set of source files for a compilation and resolves [`Span`]s against
/// them.
///
/// Files are added with [`SourceMap::add_file`], which returns a stable
/// [`FileId`]. All later phases carry `FileId`s and `Span`s and come back to the
/// map to produce human-readable locations for diagnostics.
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    /// Create an empty source map.
    #[must_use]
    pub fn new() -> SourceMap {
        SourceMap { files: Vec::new() }
    }

    /// Add a file with the given `name` and `source`, returning its [`FileId`].
    pub fn add_file(&mut self, name: impl Into<String>, source: impl Into<String>) -> FileId {
        let id = FileId::from_index(self.files.len());
        let src = source.into();
        let line_starts = compute_line_starts(&src);
        self.files.push(SourceFile {
            id,
            name: name.into(),
            src,
            line_starts,
        });
        id
    }

    /// Borrow the file with the given identifier.
    ///
    /// # Panics
    /// Panics if `id` was not produced by this map.
    #[must_use]
    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.index()]
    }

    /// All files in the map, in insertion order.
    #[must_use]
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// Resolve the start of `span` to a 1-based [`LineCol`].
    #[must_use]
    pub fn line_col(&self, span: Span) -> LineCol {
        self.file(span.file()).line_col(span.lo())
    }

    /// The source text covered by `span`.
    #[must_use]
    pub fn span_text(&self, span: Span) -> &str {
        let file = self.file(span.file());
        let src = file.source();
        let lo = span.lo().to_usize().min(src.len());
        let hi = span.hi().to_usize().min(src.len());
        &src[lo..hi]
    }
}

/// Compute the byte offset of each line start in `src`.
///
/// The result always begins with `BytePos(0)`; after every `\n`, the offset of
/// the following byte is recorded.
fn compute_line_starts(src: &str) -> Vec<BytePos> {
    let mut starts = vec![BytePos(0)];
    for (offset, byte) in src.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(BytePos(offset as u32 + 1));
        }
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with(src: &str) -> (SourceMap, FileId) {
        let mut map = SourceMap::new();
        let id = map.add_file("test.ae", src);
        (map, id)
    }

    #[test]
    fn add_file_assigns_sequential_ids() {
        let mut map = SourceMap::new();
        let a = map.add_file("a.ae", "");
        let b = map.add_file("b.ae", "");
        assert_ne!(a, b);
        assert_eq!(map.file(a).name(), "a.ae");
        assert_eq!(map.file(b).name(), "b.ae");
        assert_eq!(map.files().len(), 2);
    }

    #[test]
    fn line_col_on_single_line() {
        let (map, id) = map_with("abc");
        let f = map.file(id);
        assert_eq!(f.line_col(BytePos(0)), LineCol { line: 1, col: 1 });
        assert_eq!(f.line_col(BytePos(2)), LineCol { line: 1, col: 3 });
        // End-of-file position resolves just past the last character.
        assert_eq!(f.line_col(BytePos(3)), LineCol { line: 1, col: 4 });
    }

    #[test]
    fn line_col_across_multiple_lines() {
        // "ab\ncd\nef"
        let (map, id) = map_with("ab\ncd\nef");
        let f = map.file(id);
        assert_eq!(f.line_col(BytePos(0)), LineCol { line: 1, col: 1 }); // 'a'
        assert_eq!(f.line_col(BytePos(3)), LineCol { line: 2, col: 1 }); // 'c'
        assert_eq!(f.line_col(BytePos(6)), LineCol { line: 3, col: 1 }); // 'e'
        assert_eq!(f.line_col(BytePos(7)), LineCol { line: 3, col: 2 }); // 'f'
    }

    #[test]
    fn position_at_newline_belongs_to_its_line() {
        let (map, id) = map_with("ab\ncd");
        let f = map.file(id);
        // Byte 2 is the '\n' terminating line 1.
        assert_eq!(f.line_col(BytePos(2)), LineCol { line: 1, col: 3 });
        // Byte 3 is the start of line 2.
        assert_eq!(f.line_col(BytePos(3)), LineCol { line: 2, col: 1 });
    }

    #[test]
    fn line_text_excludes_terminators() {
        let (map, id) = map_with("ab\ncd\n");
        let f = map.file(id);
        assert_eq!(f.line_text(0), "ab");
        assert_eq!(f.line_text(1), "cd");
        // Trailing newline yields a final empty line.
        assert_eq!(f.line_text(2), "");
        assert_eq!(f.line_count(), 3);
    }

    #[test]
    fn line_text_trims_carriage_return_for_crlf() {
        let (map, id) = map_with("ab\r\ncd");
        let f = map.file(id);
        assert_eq!(f.line_text(0), "ab");
        assert_eq!(f.line_text(1), "cd");
        // The '\r' does not shift the column of the next line.
        assert_eq!(f.line_col(BytePos(4)), LineCol { line: 2, col: 1 }); // 'c'
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        // "héllo": h, é (2 bytes: 0xC3 0xA9), l, l, o
        let (map, id) = map_with("héllo");
        let f = map.file(id);
        // Byte 3 is the first 'l', the 3rd character.
        assert_eq!(f.line_col(BytePos(3)), LineCol { line: 1, col: 3 });
    }

    #[test]
    fn line_col_floors_to_char_boundary() {
        // Position inside the 'é' (byte 2) floors back to the 'é' start (byte 1).
        let (map, id) = map_with("héllo");
        let f = map.file(id);
        assert_eq!(f.line_col(BytePos(2)), LineCol { line: 1, col: 2 });
    }

    #[test]
    fn empty_and_blank_lines() {
        let (map, id) = map_with("a\n\nb");
        let f = map.file(id);
        assert_eq!(f.line_text(0), "a");
        assert_eq!(f.line_text(1), "");
        assert_eq!(f.line_text(2), "b");
        assert_eq!(f.line_col(BytePos(2)), LineCol { line: 2, col: 1 });
    }

    #[test]
    fn empty_file_has_one_line() {
        let (map, id) = map_with("");
        let f = map.file(id);
        assert_eq!(f.line_count(), 1);
        assert_eq!(f.line_text(0), "");
        assert_eq!(f.line_col(BytePos(0)), LineCol { line: 1, col: 1 });
    }

    #[test]
    fn span_text_returns_covered_source() {
        let (map, id) = map_with("let x = 5");
        let span = Span::new(id, BytePos(4), BytePos(5));
        assert_eq!(map.span_text(span), "x");
    }
}
