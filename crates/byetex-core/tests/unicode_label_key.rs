//! A label key containing non-ASCII letters (e.g. `tischrödi`, `eq_möbius`)
//! must stay a real Typst label, not get escaped into literal text.
//!
//! `sanitize_label_key` preserves Unicode alphanumerics, so the emitted label
//! is `<tischrödi>`. But `post_process_typography`'s `<…>` guard used to accept
//! only ASCII label chars, so it escaped `<tischrödi>` → `\<tischrödi>` (literal
//! text). The matching `@tischrödi` reference then aborted compilation with
//! "label does not exist". arXiv:2605.22738 hit this (now fixed); 2605.22584
//! also had two `ö` labels fixed here, though it still fails for an unrelated
//! comma-in-label-name bug (`\label{calc_annihi,crea}`).

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// A display-math label with a non-ASCII letter must be emitted as a real
/// label `<…>`, never escaped to `\<…>`.
#[test]
fn unicode_math_label_is_not_escaped() {
    let src = "\\begin{align}\\label{tischrödi}\nH\\psi = E\\psi\n\\end{align}\n\
        See \\eqref{tischrödi}.";
    let t = typst(src);
    assert!(
        t.contains("<tischrödi>"),
        "the unicode label must be emitted;\noutput:\n{t}"
    );
    assert!(
        !t.contains("\\<tischrödi>"),
        "the unicode label must NOT be escaped to literal text;\noutput:\n{t}"
    );
    assert!(
        t.contains("@tischrödi"),
        "the reference must resolve to the same key;\noutput:\n{t}"
    );
}

/// Underscore + non-ASCII combined (`eq_möbius_conversion`).
#[test]
fn unicode_label_with_underscore_not_escaped() {
    let src = "\\begin{align}\\label{eq_möbius_conversion}\nm = 1\n\\end{align}\n\
        See \\cref{eq_möbius_conversion}.";
    let t = typst(src);
    assert!(
        t.contains("<eq_möbius_conversion>") && !t.contains("\\<eq_möbius_conversion>"),
        "unicode+underscore label must stay a real label;\noutput:\n{t}"
    );
}

/// Non-regression: a genuine non-label angle span (`<email@host>`) must still be
/// escaped — the `@` is not a valid label char.
#[test]
fn email_angle_span_still_escaped() {
    let src = "Contact <foo@bar.com> for details.";
    let t = typst(src);
    assert!(
        t.contains("\\<"),
        "an angle span containing `@` must still be escaped;\noutput:\n{t}"
    );
}
