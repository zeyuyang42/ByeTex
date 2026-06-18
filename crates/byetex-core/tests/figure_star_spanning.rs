//! In a two-column layout a `figure*` / `table*` spans BOTH columns. ByeTex
//! emitted it as a plain single-column `#figure(...)`, so wide floats overflowed
//! a column (dogfood backlog: agent had to add `placement: top, scope: "parent"`
//! by hand on 2605.31564). Starred floats now emit a parent-scope spanning
//! `#place(...)` wrapper when the document is two-column.
//!
//! Robustness note: a two-column doc also emits an (often empty) parent-scope
//! place for the spanning title. To prove the FIGURE is wrapped we match the
//! marker `float: true)[\n  #figure` — the place opened immediately onto a
//! figure — which the empty title place never produces.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const FIGURE_WRAPPED: &str = "float: true)[\n  #figure";

const FIG_STAR_2COL: &str = r"\documentclass[twocolumn]{article}\usepackage{graphicx}\begin{document}
\begin{figure*}\centering\includegraphics{img}\caption{Wide.}\label{fig:w}\end{figure*}
\end{document}";

#[test]
fn figure_star_spans_in_two_column() {
    let t = typ(FIG_STAR_2COL);
    assert!(
        t.contains(FIGURE_WRAPPED),
        "figure* must be wrapped in a parent-scope spanning place; got:\n{t}"
    );
}

#[test]
fn figure_star_keeps_its_label_referenceable() {
    let t = typ(FIG_STAR_2COL);
    assert!(t.contains("<fig:w>"), "label must survive on the spanning figure; got:\n{t}");
    assert!(t.contains("caption: [Wide.]"), "caption preserved; got:\n{t}");
}

#[test]
fn plain_figure_not_wrapped_in_two_column() {
    let src = r"\documentclass[twocolumn]{article}\usepackage{graphicx}\begin{document}
\begin{figure}\centering\includegraphics{img}\caption{Narrow.}\end{figure}
\end{document}";
    let t = typ(src);
    assert!(
        !t.contains(FIGURE_WRAPPED),
        "non-starred figure must stay single-column; got:\n{t}"
    );
}

#[test]
fn figure_star_not_wrapped_in_one_column() {
    let src = r"\documentclass{article}\usepackage{graphicx}\begin{document}
\begin{figure*}\centering\includegraphics{img}\caption{Wide.}\end{figure*}
\end{document}";
    let t = typ(src);
    assert!(
        !t.contains(FIGURE_WRAPPED),
        "in one-column mode figure* needs no spanning wrapper; got:\n{t}"
    );
}

#[test]
fn table_star_spans_in_two_column() {
    let src = r"\documentclass[twocolumn]{article}\begin{document}
\begin{table*}\centering\begin{tabular}{ll}a & b\\\end{tabular}\caption{T.}\end{table*}
\end{document}";
    let t = typ(src);
    assert!(
        t.contains(FIGURE_WRAPPED),
        "table* must span in two-column; got:\n{t}"
    );
}
