//! `p{width}`/`m`/`b` paragraph columns carry a fixed width that ByeTex dropped
//! (the column became a plain left-aligned auto column → wrong layout, text
//! never wrapped to the intended width; corpus: 7 papers). The width should
//! become a Typst `columns: (…)` entry.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn p_column_width_emits_columns_tuple() {
    let t = typ(r"\begin{tabular}{p{3cm}c}A & B \\ \end{tabular}");
    assert!(t.contains("columns: (3cm, auto)"), "got:\n{t}");
}

#[test]
fn textwidth_fraction_becomes_percent() {
    let t = typ(r"\begin{tabular}{p{0.3\textwidth}l}A & B \\ \end{tabular}");
    assert!(t.contains("columns: (30%, auto)"), "got:\n{t}");
}

#[test]
fn plain_spec_keeps_integer_count_form() {
    let t = typ(r"\begin{tabular}{lcc}A & B & C \\ \end{tabular}");
    assert!(
        t.contains("columns: 3"),
        "plain spec should stay a count; got:\n{t}"
    );
    assert!(!t.contains("columns: ("), "got:\n{t}");
}
