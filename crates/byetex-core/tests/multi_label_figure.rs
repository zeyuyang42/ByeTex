//! A single float (figure/table) can carry several `\label`s — a main label
//! plus subfigure labels, or two `\captionof` blocks. Typst keeps one label
//! per element, so emit_figure attaches the referenced alias and emits a
//! hidden, referenceable anchor for every *other* label that is `\ref`'d.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn figure_attaches_the_referenced_label() {
    // Two labels, only the second is referenced → attach the second.
    let t = typ(
        "\\begin{figure}\\includegraphics{x.png}\\caption{C}\\label{fig:a}\\label{fig:b}\\end{figure}\n\nSee \\ref{fig:b}.",
    );
    assert!(
        t.contains("<fig:b>"),
        "referenced label must be attached; got:\n{t}"
    );
}

#[test]
fn figure_extra_referenced_label_gets_hidden_anchor() {
    // Both labels referenced → one on the figure, the other on a hidden anchor.
    let t = typ(
        "\\begin{figure}\\includegraphics{x.png}\\caption{C}\\label{fig:a}\\label{fig:b}\\end{figure}\n\nSee \\ref{fig:a} and \\ref{fig:b}.",
    );
    assert!(
        t.contains("<fig:a>") && t.contains("<fig:b>"),
        "both referenced labels must be present; got:\n{t}"
    );
    assert!(
        t.contains("#hide[#figure([])"),
        "the extra referenced label must use a hidden anchor; got:\n{t}"
    );
}

#[test]
fn figure_single_label_emits_no_anchor() {
    let t = typ(
        "\\begin{figure}\\includegraphics{x.png}\\caption{C}\\label{fig:a}\\end{figure}\n\nSee \\ref{fig:a}.",
    );
    assert!(t.contains("<fig:a>"), "single label attached; got:\n{t}");
    assert!(
        !t.contains("#hide"),
        "no anchor for a single label; got:\n{t}"
    );
}

#[test]
fn figure_unreferenced_extra_label_is_dropped_not_anchored() {
    // Only the first label is referenced → no anchor for the unreferenced one.
    let t = typ(
        "\\begin{figure}\\includegraphics{x.png}\\caption{C}\\label{fig:a}\\label{fig:b}\\end{figure}\n\nSee \\ref{fig:a}.",
    );
    assert!(
        t.contains("<fig:a>"),
        "referenced label attached; got:\n{t}"
    );
    assert!(
        !t.contains("<fig:b>"),
        "unreferenced extra label must not be anchored; got:\n{t}"
    );
}
