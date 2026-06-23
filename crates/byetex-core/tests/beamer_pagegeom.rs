//! Beamer presentation page geometry: with the touying emitter (Phase 3a) the slide
//! page, font, and chrome are owned by `metropolis-theme`, not a hand-rolled
//! `#set page(paper: "presentation-…")` + `#set text(22pt)` neutral preamble. A beamer
//! deck must therefore emit the touying scaffold and NOT the article us-letter layout.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str =
    "\\documentclass{beamer}\\title{T}\\begin{document}\\begin{frame}{S}x\\end{frame}\\end{document}";

#[test]
fn beamer_uses_touying_theme_not_article_page() {
    let t = typ(DECK);
    // touying owns the page geometry via the theme show-rule.
    assert!(
        t.contains("#show: metropolis-theme.with("),
        "a touying metropolis slide deck; got:\n{t}"
    );
    assert!(!t.contains("us-letter"), "not the article us-letter page; got:\n{t}");
    // No leftover hand-rolled slide page from the old neutral preamble.
    assert!(
        !t.contains("#set page(paper: \"presentation-"),
        "touying owns the page, not a plain `#set page`; got:\n{t}"
    );
}

#[test]
fn beamer_no_neutral_text_and_par_rules() {
    let t = typ(DECK);
    // The old neutral-preamble slide font / ragged-right rules are gone — touying's
    // theme sets the slide typography itself.
    assert!(!t.contains("size: 22pt"), "no hand-rolled slide font; got:\n{t}");
    assert!(!t.contains("justify: false"), "no hand-rolled par rules; got:\n{t}");
    assert!(!t.contains("first-line-indent: 1.2em"), "slides don't paragraph-indent");
}

#[test]
fn twocolumn_beamer_is_not_page_two_column() {
    // Code-review: `[twocolumn]{beamer}` must NOT make the title a parent-scoped
    // float (touying owns the layout; a `columns: 2` page float would be wrong).
    let t = typ("\\documentclass[twocolumn]{beamer}\\title{T}\\begin{document}\\begin{frame}{S}x\\end{frame}\\end{document}");
    assert!(!t.contains("columns: 2"), "beamer is never page-two-column; got:\n{t}");
    assert!(!t.contains("scope: \"parent\""), "no parent-scoped float title on a slide; got:\n{t}");
}

#[test]
fn non_beamer_keeps_article_page() {
    let t = typ("\\documentclass{article}\\begin{document}x\\end{document}");
    assert!(t.contains("us-letter"), "article keeps us-letter; got:\n{t}");
    assert!(!t.contains("metropolis-theme"), "article is not a slide deck; got:\n{t}");
}
