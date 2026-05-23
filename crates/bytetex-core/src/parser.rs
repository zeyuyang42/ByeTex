//! Thin wrapper over the vendored tree-sitter-latex C grammar.

use tree_sitter::{Language, Parser, Tree};

extern "C" {
    fn tree_sitter_latex() -> Language;
}

/// Parse a LaTeX source string into a tree-sitter [`Tree`].
///
/// The grammar's `has_error` flag is preserved on the returned tree; callers
/// inspect it to emit `ParseError` warnings.
pub fn parse(source: &str) -> Tree {
    let language = unsafe { tree_sitter_latex() };
    let mut parser = Parser::new();
    parser.set_language(&language).expect(
        "vendored tree-sitter-latex grammar is compatible with the linked tree-sitter runtime",
    );
    parser
        .parse(source, None)
        .expect("tree-sitter parse never returns None for in-memory UTF-8 input")
}
