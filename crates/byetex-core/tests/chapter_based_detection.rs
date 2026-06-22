//! `chapter_based` detection (health-check P1): the document's heading hierarchy depends on
//! whether it is chapter-bearing. This was inferred from the class NAME via a brittle
//! substring heuristic — `booklet`/`workbook` (contain "book") were wrongly treated as
//! chapter-based, and a custom chapter class with an unrecognized name was wrongly treated
//! as an article (losing level-2 sections, ToC, and front/main-matter page numbering).
//!
//! The robust signal is whether the source actually uses `\chapter`. These tests pin both
//! the fixed false positives and the newly-handled custom classes.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn booklet_without_chapter_is_not_chapter_based() {
    // `booklet` contains "book" but is NOT chapter-bearing; a \section must stay level 1.
    let t = typ("\\documentclass{booklet}\\begin{document}\\section{S}body\\end{document}");
    assert!(t.contains("= S"), "booklet \\section is level 1; got:\n{t}");
    assert!(!t.contains("== S"), "booklet \\section must NOT be level 2; got:\n{t}");
}

#[test]
fn workbook_without_chapter_is_not_chapter_based() {
    let t = typ("\\documentclass{workbook}\\begin{document}\\section{S}body\\end{document}");
    assert!(t.contains("= S") && !t.contains("== S"), "workbook \\section is level 1; got:\n{t}");
}

#[test]
fn custom_class_using_chapter_is_chapter_based() {
    // An unrecognized custom class name (no book/report/thesis token) that actually uses
    // \chapter must be treated as chapter-bearing: section at level 2, ToC + frontmatter fire.
    let t = typ("\\documentclass{floofy}\\begin{document}\\frontmatter\\tableofcontents\\chapter{Ch}\\section{S}\\mainmatter body\\end{document}");
    assert!(t.contains("== S"), "custom-class \\section under \\chapter is level 2; got:\n{t}");
    assert!(t.contains("#outline("), "custom-class \\tableofcontents renders; got:\n{t}");
    assert!(t.contains("#set page(numbering: \"i\")"), "custom-class \\frontmatter page numbering; got:\n{t}");
}

#[test]
fn known_book_class_unchanged() {
    // Regression: an exact known chapter class keeps level-2 sections even without scanning.
    let t = typ("\\documentclass{book}\\begin{document}\\chapter{Ch}\\section{S}\\subsection{Sub}body\\end{document}");
    assert!(t.contains("= Ch") && t.contains("== S") && t.contains("=== Sub"), "book hierarchy intact; got:\n{t}");
}

#[test]
fn article_unchanged() {
    // Regression: article stays level-1 sections, no ToC outline.
    let t = typ("\\documentclass{article}\\begin{document}\\tableofcontents\\section{S}\\subsection{Sub}body\\end{document}");
    assert!(t.contains("= S") && t.contains("== Sub"), "article hierarchy intact; got:\n{t}");
    assert!(!t.contains("#outline("), "article \\tableofcontents stays dropped; got:\n{t}");
}
