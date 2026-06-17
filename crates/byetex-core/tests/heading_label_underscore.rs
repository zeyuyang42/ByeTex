//! A `\label{...}` with an underscore attached to a heading used to leak its
//! tail as body text: tree-sitter truncates the `label` token at the first `_`,
//! so `\section{X}\label{sec:exp1_main}` emitted `= X <sec:exp1_main>` (correct
//! anchor) PLUS a stray `\_main` paragraph (corpus 2605.31586, 6 such labels,
//! dogfood backlog F4). The heading-label path now consumes the full brace span.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn underscore_label_on_heading_does_not_leak_tail() {
    let t = typ(r"\documentclass{article}\begin{document}\section{Experiments}\label{sec:exp1_main}Some text.\end{document}");
    assert!(
        !t.contains(r"\_main") && !t.contains("_main\n"),
        "label tail must not leak as body text; got:\n{t}"
    );
}

#[test]
fn underscore_label_on_heading_keeps_full_anchor() {
    let t = typ(r"\documentclass{article}\begin{document}\section{Experiments}\label{sec:exp1_main}Some text.\end{document}");
    assert!(
        t.contains("<sec:exp1_main>"),
        "full label must still anchor the heading; got:\n{t}"
    );
}

#[test]
fn underscore_label_heading_keeps_body_text() {
    let t = typ(r"\documentclass{article}\begin{document}\section{Experiments}\label{sec:exp1_main}Some text.\end{document}");
    assert!(t.contains("Some text."), "body must survive; got:\n{t}");
}

#[test]
fn multi_underscore_label_on_heading_clean() {
    let t = typ(r"\documentclass{article}\begin{document}\subsection{S}\label{sec:a_b_c_d}Body.\end{document}");
    assert!(!t.contains(r"\_b"), "no leaked underscore tail; got:\n{t}");
    assert!(t.contains("<sec:a_b_c_d>"), "full multi-underscore anchor; got:\n{t}");
}
