//! `\verb`, `\path`, `\lstinline` and `\mintinline` read their delimiter from
//! the source. LaTeX allows ANY non-letter character there, including a
//! multi-byte one (`§`, `€`, `«`). The emitter used to read the delimiter as a
//! single BYTE and then slice `&src[end + 1..close]`, which lands mid-codepoint
//! and panics the whole conversion ("byte index N is not a char boundary").
//!
//! Review finding #2. These tests assert the conversion completes and the
//! verbatim content survives.

use byetex_core::{convert, ConvertOptions};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn verb_with_multibyte_delimiter_does_not_panic() {
    let out = convert(&doc("Run \\verb§rm -rf /tmp§ now."), &ConvertOptions::default());
    assert!(
        out.typst.contains("rm -rf /tmp"),
        "verbatim content lost: {}",
        out.typst
    );
    assert!(
        !out.typst.contains('§'),
        "delimiter leaked into output: {}",
        out.typst
    );
}

#[test]
fn starred_verb_with_multibyte_delimiter_does_not_panic() {
    let out = convert(&doc("A \\verb*€a b€ B."), &ConvertOptions::default());
    assert!(out.typst.contains("a b"), "content lost: {}", out.typst);
}

#[test]
fn path_with_multibyte_delimiter_does_not_panic() {
    let out = convert(&doc("See \\path§/usr/löcal§."), &ConvertOptions::default());
    assert!(
        out.typst.contains("/usr/l"),
        "path content lost: {}",
        out.typst
    );
}

#[test]
fn lstinline_with_multibyte_delimiter_does_not_panic() {
    let out = convert(
        &doc("Call \\lstinline§foo(1)§ here."),
        &ConvertOptions::default(),
    );
    assert!(out.typst.contains("foo(1)"), "code lost: {}", out.typst);
}

#[test]
fn mintinline_with_multibyte_delimiter_does_not_panic() {
    let out = convert(
        &doc("Call \\mintinline{python}§len(x)§ here."),
        &ConvertOptions::default(),
    );
    assert!(out.typst.contains("len(x)"), "code lost: {}", out.typst);
}

#[test]
fn verb_delimiter_inside_multibyte_paragraph_still_ascii_delimited() {
    // Multi-byte text BEFORE the \verb shifts every byte offset; the ASCII
    // delimiter path must stay correct too.
    let out = convert(
        &doc("Übergröße naïve — \\verb|x_1| done."),
        &ConvertOptions::default(),
    );
    assert!(out.typst.contains("x_1"), "content lost: {}", out.typst);
}
