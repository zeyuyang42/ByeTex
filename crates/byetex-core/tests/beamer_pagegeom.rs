//! Beamer presentation page geometry (B4): a beamer deck should render on a
//! landscape 16:9 slide page with a larger base font and no justification — not the
//! us-letter 10pt portrait article layout.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str =
    "\\documentclass{beamer}\\title{T}\\begin{document}\\begin{frame}{S}x\\end{frame}\\end{document}";

#[test]
fn beamer_uses_presentation_page() {
    let t = typ(DECK);
    // A slide page (default 4:3; aspect-ratio detection covered in beamer_aspectratio).
    assert!(t.contains("presentation-"), "a slide presentation page; got:\n{t}");
    assert!(!t.contains("us-letter"), "not the article us-letter page");
}

#[test]
fn beamer_no_justify_and_larger_font() {
    let t = typ(DECK);
    assert!(t.contains("justify: false"), "slides are ragged-right; got:\n{t}");
    // A slide font is much larger than the 10pt article default.
    assert!(t.contains("size: 22pt"), "larger slide base font; got:\n{t}");
    assert!(!t.contains("first-line-indent: 1.2em"), "slides don't paragraph-indent");
}

#[test]
fn twocolumn_beamer_is_not_page_two_column() {
    // Code-review: `[twocolumn]{beamer}` must NOT make the title a parent-scoped
    // float (the slide page has no `columns: 2` context → Typst would error).
    let t = typ("\\documentclass[twocolumn]{beamer}\\title{T}\\begin{document}\\begin{frame}{S}x\\end{frame}\\end{document}");
    assert!(!t.contains("columns: 2"), "beamer is never page-two-column; got:\n{t}");
    assert!(!t.contains("scope: \"parent\""), "no parent-scoped float title on a slide; got:\n{t}");
}

#[test]
fn non_beamer_keeps_article_page() {
    let t = typ("\\documentclass{article}\\begin{document}x\\end{document}");
    assert!(t.contains("us-letter"), "article keeps us-letter; got:\n{t}");
    assert!(!t.contains("presentation-16-9"), "article is not a slide deck");
}
