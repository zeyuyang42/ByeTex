//! Beamer title-slide author/affiliation styling (B-polish): a beamer title slide
//! shows author + `\institute` as plain centered lines, NOT the academic-paper
//! superscript-numbered affiliation footnoting.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = "\\documentclass{beamer}\\title{My Talk}\\author{Jane Doe}\\institute{Example University}\\date{2026}\\begin{document}\\frame{\\titlepage}\\end{document}";

#[test]
fn beamer_author_and_institute_are_plain() {
    let t = typ(DECK);
    assert!(t.contains("Jane Doe"), "author rendered");
    assert!(t.contains("Example University"), "institute rendered");
    // No academic superscript numbering on a slide.
    assert!(!t.contains("#super["), "no superscript affiliation markers; got:\n{t}");
}

#[test]
fn beamer_multi_author_no_superscripts() {
    // Two authors + institute: still plain (beamer groups/centers, no numbering).
    let t = typ("\\documentclass{beamer}\\title{T}\\author{Alice \\and Bob}\\institute{MIT}\\begin{document}\\frame{\\titlepage}\\end{document}");
    assert!(t.contains("Alice") && t.contains("Bob"), "both authors rendered; got:\n{t}");
    assert!(!t.contains("#super["), "no superscripts on a slide; got:\n{t}");
}

#[test]
fn non_beamer_keeps_superscript_affiliation() {
    // Regression: a normal paper keeps the numbered affiliation style.
    let t = typ("\\documentclass{article}\\title{T}\\author{Alice}\\affiliation{MIT}\\begin{document}\\maketitle\\end{document}");
    assert!(t.contains("Alice"), "author rendered");
    assert!(t.contains("#super["), "paper keeps superscript affiliations; got:\n{t}");
}
