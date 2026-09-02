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

#[test]
#[ignore = "open: no discriminator between a float Typst places fine and one it clamps"]
fn an_oversized_spanning_float_is_still_clamped_on_2605_31586() {
    // OPEN. `table*` in a two-column class becomes `place(..., float: true)`, and
    // a FLOATED placement cannot break across pages. So a spanning table taller
    // than the column is clamped into a pile of overprinted rows — the one case
    // the table-figure breakable rule (#523) cannot reach, because the float sits
    // outside it. 2605.31586 stacks 13 words at a single position.
    //
    // A `byetex-float` helper that drops the float when the body does not fit DOES
    // clear it (corpus pile-ups 1 -> 0, verified). It was withheld because the
    // fidelity gate rejected it: 2605.31584 went `structure_ok true->false`,
    // page_ratio 0.952 -> 0.619 (20 pages -> 13, truth 21). No content was lost
    // there (word_recall 0.848 unchanged, word_count_ratio 0.995) — floats spread
    // content across pages, and un-floating packs it back together.
    //
    // Three fit tests were tried and all three un-float 31584:
    //   * `measure(block(width: size.width, ...)).height >= size.height`
    //     — and note a BARE `measure(body)` lays out at infinite width, so no row
    //       wraps and the table reports 227pt against a 700pt container; the check
    //       then never fires at all and the helper silently does nothing.
    //   * width doubled to approximate the spanning width — still un-floats.
    //   * a fixed 600pt page-height threshold — still un-floats.
    //
    // The real obstacle: `size.height` inside `layout()` is the REMAINING space,
    // not the page height, and more importantly an oversized float does NOT always
    // clamp — Typst often moves it to its own page, which is what 31584 relies on.
    // Height alone cannot separate the two cases, so any fix needs a signal for
    // "this float will actually be clamped", not "this float is tall".
    //
    // Also required, whatever the eventual fix: mark the helper used in the PARENT
    // emitter (`used_spanning_float |= sub.used_spanning_float`, as `used_fit_width`
    // and `used_subpar` already do), or a float inside an `\input`ed file emits a
    // call to an undefined name — that regressed 3 papers and acceptance caught it.
    let _ = ();
}
