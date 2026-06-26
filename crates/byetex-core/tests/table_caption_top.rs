//! LaTeX tables put their `\caption` ABOVE the tabular (the near-universal
//! convention); Typst's `#figure` defaults the caption BELOW. A document-level
//! `#show figure.where(kind: table): set figure.caption(position: top)` moves
//! table captions above to match. Figure captions stay below. Found by the
//! visual grader on 2605.22786.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn table_captions_set_to_top() {
    let t = typ(r"\documentclass{article}\begin{document}\begin{table}\caption{T}\begin{tabular}{ll}a&b\\c&d\end{tabular}\end{table}\end{document}");
    assert!(
        t.contains("#show figure.where(kind: table): set figure.caption(position: top)"),
        "no table caption-position rule; got:\n{t}"
    );
}

#[test]
fn bare_fragment_has_no_preamble_rule() {
    // A bare fragment (no documentclass / title) stays preamble-free.
    let t = typ(r"\begin{tabular}{ll}a&b\\c&d\end{tabular}");
    assert!(
        !t.contains("figure.caption(position: top)"),
        "fragment should not get the preamble rule; got:\n{t}"
    );
}
