//! Position primitives: byte offsets, file identifiers, spans, and resolved
//! line/column locations.

/// A zero-based byte offset within a single source file.
///
/// Byte offsets (rather than character indices) are used because they map
/// directly onto slices of the source string and are what the lexer naturally
/// produces. The 32-bit width caps a single source file at 4 GiB, which is far
/// beyond any realistic input.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BytePos(pub u32);

impl BytePos {
    /// The offset as a `usize`, for slicing into source strings.
    #[must_use]
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }

    /// Construct a `BytePos` from a `usize`, truncating to 32 bits.
    ///
    /// Truncation only occurs for inputs larger than 4 GiB, which are not
    /// supported (see the type-level note).
    #[must_use]
    pub fn from_usize(offset: usize) -> BytePos {
        BytePos(offset as u32)
    }
}

/// Identifies a [`SourceFile`](crate::SourceFile) within a
/// [`SourceMap`](crate::SourceMap).
///
/// `FileId`s are opaque handles minted only by a `SourceMap`; a `FileId` is only
/// meaningful for the map that produced it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FileId(u32);

impl FileId {
    /// Create a `FileId` for the given index. Crate-internal: only the
    /// `SourceMap` assigns identifiers.
    pub(crate) fn from_index(index: usize) -> FileId {
        FileId(index as u32)
    }

    /// The index this `FileId` refers to within its owning `SourceMap`.
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

/// A contiguous region of source: the half-open byte range `[lo, hi)` within a
/// single file.
///
/// `Span` is small and `Copy` so it can be attached to every token, AST node,
/// and IR value cheaply. Its fields are private; construct one with [`Span::new`]
/// and inspect it through the accessors. Keeping the representation private lets
/// it be optimized (e.g. rustc-style packing) later without breaking callers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    file: FileId,
    lo: BytePos,
    hi: BytePos,
}

impl Span {
    /// Create a span covering `[lo, hi)` in `file`.
    ///
    /// # Panics
    /// Panics (in debug builds) if `lo > hi`.
    #[must_use]
    pub fn new(file: FileId, lo: BytePos, hi: BytePos) -> Span {
        debug_assert!(lo <= hi, "span lo ({lo:?}) must not exceed hi ({hi:?})");
        Span { file, lo, hi }
    }

    /// The file this span points into.
    #[must_use]
    pub fn file(self) -> FileId {
        self.file
    }

    /// The inclusive start of the span.
    #[must_use]
    pub fn lo(self) -> BytePos {
        self.lo
    }

    /// The exclusive end of the span.
    #[must_use]
    pub fn hi(self) -> BytePos {
        self.hi
    }

    /// The length of the span in bytes.
    #[must_use]
    pub fn len(self) -> u32 {
        self.hi.0 - self.lo.0
    }

    /// Whether the span is empty (`lo == hi`), as for an end-of-file marker.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.lo == self.hi
    }

    /// The smallest span covering both `self` and `other`.
    ///
    /// This is the workhorse for building the span of a compound construct from
    /// the spans of its parts (e.g. a binary expression from its operands).
    ///
    /// # Panics
    /// Panics if the two spans are in different files; merging across files is a
    /// bug.
    #[must_use]
    pub fn to(self, other: Span) -> Span {
        assert_eq!(
            self.file, other.file,
            "cannot merge spans from different files"
        );
        Span {
            file: self.file,
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }
}

/// A resolved, human-facing location: a 1-based line and 1-based column.
///
/// The column counts Unicode scalar values (characters), not bytes, so it is
/// correct for UTF-8 source. Display width (double-width or zero-width glyphs) is
/// a future refinement; for now every character counts as one column.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LineCol {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column, counted in characters.
    pub col: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file0() -> FileId {
        FileId::from_index(0)
    }

    #[test]
    fn byte_pos_conversions_round_trip() {
        assert_eq!(BytePos::from_usize(42).to_usize(), 42);
        assert_eq!(BytePos(7).to_usize(), 7);
    }

    #[test]
    fn file_ids_are_distinct_by_index() {
        assert_ne!(FileId::from_index(0), FileId::from_index(1));
        assert_eq!(FileId::from_index(3).index(), 3);
    }

    #[test]
    fn span_accessors_and_length() {
        let s = Span::new(file0(), BytePos(4), BytePos(9));
        assert_eq!(s.file(), file0());
        assert_eq!(s.lo(), BytePos(4));
        assert_eq!(s.hi(), BytePos(9));
        assert_eq!(s.len(), 5);
        assert!(!s.is_empty());
    }

    #[test]
    fn empty_span_reports_empty() {
        let s = Span::new(file0(), BytePos(3), BytePos(3));
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn span_merge_covers_both() {
        let a = Span::new(file0(), BytePos(2), BytePos(5));
        let b = Span::new(file0(), BytePos(8), BytePos(10));
        let merged = a.to(b);
        assert_eq!(merged.lo(), BytePos(2));
        assert_eq!(merged.hi(), BytePos(10));
        // Merging is order-independent.
        assert_eq!(b.to(a), merged);
    }

    #[test]
    #[should_panic(expected = "different files")]
    fn span_merge_across_files_panics() {
        let a = Span::new(FileId::from_index(0), BytePos(0), BytePos(1));
        let b = Span::new(FileId::from_index(1), BytePos(0), BytePos(1));
        let _ = a.to(b);
    }
}
