//! Regression tests for `#raw(...)` content inside table cells.
//!
//! Table cells are markup-escaped by `escape_text_cell` (e.g. `_`→`\_`). That
//! pass must NOT touch the contents of a `#raw("...")` call: `#raw("a_b")` is
//! Typst code whose string literal is verbatim, so escaping the underscore to
//! `\_` corrupts it (renders a stray backslash; `\_` is not a Typst string
//! escape). Affects `\verb`, `\texttt`, and `\path` cells alike.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// `\verb|a_b|` in a table cell must keep `#raw("a_b")` — no `\_` corruption.
#[test]
fn verb_underscore_in_table_cell_not_escaped() {
    let t = typst("\\begin{tabular}{ll}\n\\verb|a_b| & x \\\\\n\\end{tabular}");
    assert!(
        t.contains("#raw(\"a_b\")"),
        "verb cell must keep clean #raw(\"a_b\");\noutput:\n{t}"
    );
    assert!(
        !t.contains("a\\_b"),
        "underscore inside #raw(...) must not be escaped to \\_;\noutput:\n{t}"
    );
}

/// `\path|feat_name|` in a table cell must keep `#raw("feat_name")`.
#[test]
fn path_underscore_in_table_cell_not_escaped() {
    let t = typst("\\begin{tabular}{ll}\n\\path|feat_name| & 0.5 \\\\\n\\end{tabular}");
    assert!(
        t.contains("#raw(\"feat_name\")"),
        "path cell must keep clean #raw(\"feat_name\");\noutput:\n{t}"
    );
}

/// `\texttt{a_b}` in a table cell must keep its raw underscore.
#[test]
fn texttt_underscore_in_table_cell_not_escaped() {
    let t = typst("\\begin{tabular}{ll}\n\\texttt{a_b} & y \\\\\n\\end{tabular}");
    assert!(
        t.contains("#raw(\"a_b\")"),
        "texttt cell must keep clean #raw(\"a_b\");\noutput:\n{t}"
    );
}

/// Plain markup text in a cell must STILL be escaped (no regression): a bare
/// `_` outside `#raw(...)` becomes `\_`.
#[test]
fn plain_cell_underscore_still_escaped() {
    let t = typst("\\begin{tabular}{ll}\na_b & x \\\\\n\\end{tabular}");
    assert!(
        t.contains("a\\_b"),
        "bare underscore in a plain cell must still be escaped to \\_;\noutput:\n{t}"
    );
}

/// A `#raw("...")` whose content contains an escaped quote/backslash is copied
/// verbatim and the closing `)` is found correctly (the `)` does not terminate
/// the call early, and trailing markup is still escaped).
#[test]
fn raw_with_escaped_quote_then_markup() {
    // `\verb` with a literal `"` and `_`, followed by a plain `_b` markup cell.
    let t = typst("\\begin{tabular}{ll}\n\\verb|a\"b_c| & d_e \\\\\n\\end{tabular}");
    // raw content preserved verbatim (escaped quote + raw underscore)
    assert!(
        t.contains("#raw(\"a\\\"b_c\")"),
        "raw content with escaped quote must be verbatim;\noutput:\n{t}"
    );
    // the following plain cell still gets its underscore escaped
    assert!(
        t.contains("d\\_e"),
        "markup after the #raw(...) call must still be escaped;\noutput:\n{t}"
    );
}
