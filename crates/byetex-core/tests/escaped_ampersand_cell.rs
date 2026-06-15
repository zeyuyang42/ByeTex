//! An escaped `\&` (literal ampersand) inside a table cell was treated as a
//! column separator, splitting the cell and leaving an unclosed `[...]`
//! (corpus 2605.31604: `\multicolumn{3}{c}{Document \& Diagram}` → an unclosed
//! `table.cell(colspan: 3)[Document` → 98 cascading "unclosed delimiter" errors).
//! Fix: render `\&` as `\&` (keeps the backslash) and split cells on *unescaped*
//! `&` only.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn escaped_amp_in_multicolumn_is_one_cell() {
    let src = "\\begin{tabular}{ccc}\n\
        a & \\multicolumn{2}{c}{Document \\& Diagram} \\\\\n\
        \\end{tabular}";
    let t = typst(src);
    assert!(
        t.contains("colspan: 2)[Document \\& Diagram]"),
        "escaped \\& must stay inside one cell, not split it;\noutput:\n{t}"
    );
}

#[test]
fn escaped_amp_does_not_split_plain_cell() {
    let src = "\\begin{tabular}{cc}\n A \\& B & C \\\\\n\\end{tabular}";
    let t = typst(src);
    // The row has exactly ONE real separator → two cells: `A \& B` and `C`.
    assert!(
        t.contains("[A \\& B], [C]"),
        "row must split into [A \\& B] and [C] only;\noutput:\n{t}"
    );
}

#[test]
fn escaped_amp_renders_as_escaped_in_text() {
    let t = typst(r"R\&D budget");
    assert!(
        t.contains("R\\&D"),
        "text-mode \\& must render as \\&;\noutput:\n{t}"
    );
}
