//! A single-label `\begin{equation}` referenced by `\ref`/`\eqref` must be
//! numbered, or Typst errors "cannot reference equation without numbering"
//! (corpus 2605.31603). ByeTex only enabled equation numbering for multi-label
//! equations; now it also enables it when an equation's label is referenced.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn referenced_single_label_equation_enables_numbering() {
    let src = "\\begin{equation}\\label{LF_eq}\nx = 1\n\\end{equation}\n\
        See Eq.~\\ref{LF_eq}.";
    let t = typst(src);
    assert!(
        t.contains("#set math.equation(numbering:"),
        "a referenced equation must enable equation numbering;\noutput:\n{t}"
    );
    assert!(
        t.contains("@LF_eq"),
        "the reference must be emitted;\noutput:\n{t}"
    );
}

#[test]
fn unreferenced_equation_stays_unnumbered() {
    // Non-regression: an equation nobody references must NOT force numbering.
    let src = "\\begin{equation}\nx = 1\n\\end{equation}\nPlain text.";
    let t = typst(src);
    assert!(
        !t.contains("#set math.equation(numbering:"),
        "an unreferenced equation must not enable numbering;\noutput:\n{t}"
    );
}
