//! `\captionof{TYPE}{caption}` (caption package) captions content that isn't
//! in a matching float — common inside a `figure`/`table` env or a minipage.
//! ByeTex dropped it, so the caption text and any following `\label` were
//! lost and `\ref` to that figure dangled. Treat it as a caption source
//! (its 2nd arg is the caption; the 1st arg, the type, picks the kind).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn captionof_figure_provides_caption_in_figure_env() {
    let t = typ(
        "\\begin{figure}\n\\includegraphics{x.png}\n\\captionof{figure}{My caption}\\label{fig:x}\n\\end{figure}",
    );
    assert!(
        t.contains("caption: [My caption]"),
        "captionof's text must become the figure caption; got:\n{t}"
    );
    assert!(
        t.contains("<fig:x>"),
        "the label must attach to the figure; got:\n{t}"
    );
}

#[test]
fn captionof_table_sets_table_kind() {
    let t = typ(
        "\\begin{table}\n\\begin{tabular}{cc}a & b\\end{tabular}\n\\captionof{table}{Tab cap}\\label{tab:y}\n\\end{table}",
    );
    assert!(
        t.contains("caption: [Tab cap]"),
        "captionof table caption must be used; got:\n{t}"
    );
    assert!(
        t.contains("kind: table"),
        "\\captionof{{table}} must set kind: table so refs read 'Table N'; got:\n{t}"
    );
}

#[test]
fn explicit_caption_still_wins_over_captionof() {
    // Regression: a real \caption is preferred when both are present.
    let t = typ(
        "\\begin{figure}\n\\includegraphics{x.png}\n\\caption{Real}\\label{fig:r}\n\\end{figure}",
    );
    assert!(
        t.contains("caption: [Real]"),
        "explicit \\caption used; got:\n{t}"
    );
}
