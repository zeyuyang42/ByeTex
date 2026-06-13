//! `\multirow{-N}{..}{..}` (negative = span upward in LaTeX) emitted
//! `table.cell(rowspan: -N)`, which Typst rejects ("number must be positive"),
//! blocking compilation (corpus 2605.31563). Fix: emit `max(1, |N|)`.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn negative_multirow_emits_positive_rowspan() {
    let src = "\\begin{tabular}{ll}\n\\multirow{-2}{*}{Hard} & A \\\\\n & B \\\\\n\\end{tabular}";
    let t = typst(src);
    assert!(t.contains("rowspan: 2"), "negative multirow → |N|;\noutput:\n{t}");
    assert!(!t.contains("rowspan: -2"), "must not emit a negative rowspan;\noutput:\n{t}");
}

#[test]
fn positive_multirow_unchanged() {
    let src = "\\begin{tabular}{ll}\n\\multirow{3}{*}{Top} & A \\\\\n & B \\\\\n & C \\\\\n\\end{tabular}";
    let t = typst(src);
    assert!(t.contains("rowspan: 3"), "positive multirow preserved;\noutput:\n{t}");
}
