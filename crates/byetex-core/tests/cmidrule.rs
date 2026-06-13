//! `\cmidrule[width](trim){a-b}` (booktabs) draws a partial horizontal rule
//! under columns a..b. ByeTex dropped the `\cmidrule` token but left its
//! `(trim){a-b}` args, which leaked into the following cell (content
//! corruption), and the rule itself was lost (corpus: 23 papers). The args must
//! be consumed, and the rule emitted as a partial `table.hline(start, end)`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const T: &str = r"\begin{tabular}{lcc}\toprule N & A & B \\ \midrule X & 1 & 2 \\ \cmidrule(lr){2-3} Y & 3 & 4 \\ \bottomrule \end{tabular}";

#[test]
fn cmidrule_args_do_not_leak_into_cells() {
    let t = typ(T);
    assert!(!t.contains("(lr)"), "trim spec leaked; got:\n{t}");
    assert!(!t.contains("2-3"), "range arg leaked; got:\n{t}");
    assert!(t.contains("[Y]"), "the Y cell must be clean; got:\n{t}");
}

#[test]
fn cmidrule_emits_partial_hline() {
    // `{2-3}` (1-indexed, inclusive) → Typst columns [1, 3) i.e. start: 1, end: 3.
    let t = typ(T);
    assert!(
        t.contains("table.hline(start: 1, end: 3"),
        "partial rule under cols 2-3 missing; got:\n{t}"
    );
}
