//! `\includegraphics` NOT inside a `\begin{figure}` float (standalone in body,
//! or in a `center` block — common for teaser figures) emitted a bare
//! `image(...)` with no `#`, so Typst treated it as a string literal and DROPPED
//! the image. Standalone images now get the `#`; float-body images stay bare
//! (the `#figure(...)` wrapper provides the sigil). Found by the visual grader
//! on 2605.31597 (LNCS teaser figure absent).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn standalone_includegraphics_is_a_call() {
    let t = typ(r"Text \includegraphics{fig.jpg} more.");
    assert!(t.contains("#image("), "standalone image dropped (no #); got:\n{t}");
}

#[test]
fn includegraphics_in_center_renders() {
    let t = typ(r"\begin{center}\includegraphics{fig.jpg}\end{center}");
    assert!(t.contains("#image("), "center image dropped; got:\n{t}");
}

#[test]
fn figure_body_image_is_not_double_hashed() {
    let t = typ(r"\begin{figure}\includegraphics{fig.jpg}\caption{C}\end{figure}");
    assert!(t.contains("#figure("), "figure not emitted; got:\n{t}");
    assert!(!t.contains("figure(\n  #image") && !t.contains("figure(#image"),
        "figure body has a stray # before image; got:\n{t}");
}
