//! A comma inside a single-key reference (`\ref`/`\eqref`/`\pageref`/`\autoref`)
//! is part of a literal label name, not a cleveref multi-key separator. The
//! definition `\label{calc_annihi,crea}` sanitizes the comma to `-`
//! (`<calc_annihi-crea>`); the reference must do the same so the two match.
//! Only cleveref commands (`\cref`/`\Cref`) split on comma. arXiv:2605.22584
//! hit this (`\eqref{calc_annihi,crea}`).

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// `\eqref{a,b}` references ONE label named `a,b` → `(@a-b)`, matching the
/// `\label{a,b}` definition `<a-b>`. It must NOT split into two refs.
#[test]
fn eqref_comma_is_single_literal_key() {
    let src = "\\begin{align}\\label{calc_annihi,crea}\nx = 1\n\\end{align}\n\
        See \\eqref{calc_annihi,crea}.";
    let t = typst(src);
    assert!(
        t.contains("<calc_annihi-crea>"),
        "the label key must sanitize the comma to `-`;\noutput:\n{t}"
    );
    assert!(
        t.contains("@calc_annihi-crea"),
        "the reference must use the same single sanitized key;\noutput:\n{t}"
    );
    // Must NOT have split into two separate references.
    assert!(
        !t.contains("@calc_annihi,") && !t.contains("@crea"),
        "a single-key \\eqref must not split on the comma;\noutput:\n{t}"
    );
}

/// Plain `\ref{a,b}` is likewise a single literal key.
#[test]
fn ref_comma_is_single_literal_key() {
    let src = "\\section{S}\\label{sec,one}\nSee \\ref{sec,one}.";
    let t = typst(src);
    assert!(
        t.contains("@sec-one") && !t.contains("@one"),
        "a single-key \\ref must not split on the comma;\noutput:\n{t}"
    );
}

/// Non-regression: cleveref `\cref{a,b}` STILL splits into two references
/// (Bug #45 behavior preserved).
#[test]
fn cref_still_splits_on_comma() {
    let src = "\\section{A}\\label{sec:a}\n\\section{B}\\label{sec:b}\n\
        See \\cref{sec:a,sec:b}.";
    let t = typst(src);
    assert!(
        t.contains("@sec:a") && t.contains("@sec:b"),
        "\\cref must still split into two references;\noutput:\n{t}"
    );
}
