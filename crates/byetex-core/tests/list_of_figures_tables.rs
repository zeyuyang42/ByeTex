//! `\listoffigures` / `\listoftables` in book-family classes.
//!
//! Both were dropped with a warning. On gh-amberj-latex-book-template the LaTeX
//! truth gives each its own page ("List of Figures vii", "List of Tables ...")
//! while our output contained neither — part of why that book renders 6 pages
//! against the truth's 16 with near-identical token counts (1571 vs 1534).
//!
//! Scoped to chapter-bearing classes, matching how `\tableofcontents` is already
//! handled: in the article family these are rare and the drop stays.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn book(body: &str) -> String {
    format!(
        "\\documentclass{{book}}\n\\begin{{document}}\n{body}\n\\chapter{{One}}\nText.\n\\end{{document}}"
    )
}

#[test]
fn listoffigures_becomes_a_figure_outline() {
    let t = typ(&book("\\listoffigures"));
    assert!(
        t.contains("target: figure.where(kind: image)"),
        "\\listoffigures must list figures; got:\n{t}"
    );
    assert!(t.contains("List of Figures"), "with LaTeX's title; got:\n{t}");
}

#[test]
fn listoftables_becomes_a_table_outline() {
    let t = typ(&book("\\listoftables"));
    assert!(
        t.contains("target: figure.where(kind: table)"),
        "\\listoftables must list tables; got:\n{t}"
    );
    assert!(t.contains("List of Tables"), "with LaTeX's title; got:\n{t}");
}

#[test]
fn each_front_matter_list_starts_a_new_page() {
    // In the book class these are `\chapter*`, which begins a page. Truth gives
    // Contents, List of Figures and List of Tables a page each; we ran them
    // together.
    let t = typ(&book("\\tableofcontents\n\\listoffigures\n\\listoftables"));
    let breaks = t.matches("#pagebreak").count();
    assert!(
        breaks >= 2,
        "the lists must not run together on one page; got {breaks} breaks in:\n{t}"
    );
}

#[test]
fn an_article_still_drops_them() {
    // The control: the article family has no chapters and keeps the old
    // behaviour, so this cannot disturb the 59 arXiv papers.
    let t = typ("\\documentclass{article}\n\\begin{document}\n\\listoffigures\nText.\n\\end{document}");
    assert!(
        !t.contains("List of Figures"),
        "article-family behaviour is unchanged; got:\n{t}"
    );
}
