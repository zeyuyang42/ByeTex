//! IEEE classes abbreviate the figure caption label ("Fig. 1") and use an
//! all-caps, roman-numbered table label ("TABLE I"). ByeTex emitted Typst's
//! defaults ("Figure 1" / "Table 1"). Set the per-kind supplement (and roman
//! table numbering) in the IEEEtran preamble. Other classes keep the defaults.
//! Found by the visual grader on 2605.22779.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn ieeetran_sets_fig_and_table_supplements() {
    let t = typ(r"\documentclass[conference]{IEEEtran}\begin{document}\section{S}Body.\end{document}");
    assert!(t.contains("supplement: [Fig.]"), "no IEEE figure supplement; got:\n{t}");
    assert!(t.contains("supplement: [TABLE]"), "no IEEE table supplement; got:\n{t}");
}

#[test]
fn article_keeps_default_figure_labels() {
    let t = typ(r"\documentclass{article}\begin{document}\section{S}Body.\end{document}");
    assert!(
        !t.contains("supplement: [Fig.]") && !t.contains("supplement: [TABLE]"),
        "article should keep Typst default figure/table labels; got:\n{t}"
    );
}
