//! The reference-list heading: LaTeX's `\refname` vs `\bibname`.
//!
//! Typst's `#bibliography` defaults to "Bibliography". LaTeX's article classes
//! set `\refname` = "References"; only book/report classes use `\bibname` =
//! "Bibliography". We emitted the book heading for everything.
//!
//! Measured: 30 of 71 corpus papers show "References" in the LaTeX truth and
//! "Bibliography" in ours — the widest-reaching single mismatch found in this
//! run of the loop, and visible on the reference page of every one of them.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn bib_line(t: &str) -> String {
    t.lines()
        .find(|l| l.contains("#bibliography("))
        .unwrap_or("<no bibliography>")
        .to_string()
}

const BODY: &str = "\\begin{document}\nText \\cite{k}.\n\\bibliography{refs}\n\\end{document}";

#[test]
fn an_article_gets_references() {
    let b = bib_line(&typ(&format!("\\documentclass{{article}}\n{BODY}")));
    assert!(
        b.contains("title: [References]"),
        "article classes set \\refname = References; got: {b}"
    );
}

#[test]
fn a_book_keeps_bibliography() {
    // `\bibname` is "Bibliography" for book/report, and `\chapter` is the robust
    // signal the emitter already uses to tell them apart.
    let b = bib_line(&typ(
        "\\documentclass{book}\n\\begin{document}\n\\chapter{One}\nText \\cite{k}.\n\\bibliography{refs}\n\\end{document}",
    ));
    assert!(
        !b.contains("title: [References]"),
        "book classes keep Bibliography; got: {b}"
    );
}

#[test]
fn the_style_argument_is_preserved() {
    // The title must be added alongside the style, not instead of it.
    let b = bib_line(&typ(&format!(
        "\\documentclass{{article}}\n\\bibliographystyle{{ieeetr}}\n{BODY}"
    )));
    assert!(b.contains("style:"), "style must survive; got: {b}");
    assert!(b.contains("title: [References]"), "title must be added; got: {b}");
}

#[test]
fn a_renamed_refname_is_honoured() {
    // A document that redefines the name means it; `\renewcommand{\refname}{...}`
    // is how authors localise or retitle the section.
    let b = bib_line(&typ(&format!(
        "\\documentclass{{article}}\n\\renewcommand{{\\refname}}{{Literature}}\n{BODY}"
    )));
    assert!(b.contains("title: [Literature]"), "explicit \\refname wins; got: {b}");
}
