//! Book/report heading hierarchy (round-5 dogfood T2): in a chapter-bearing class
//! (`book`/`report`/thesis), `\chapter` is the top level and `\section` sits BELOW it
//! (level 2), not flattened to the same level as the chapter. Article is unchanged
//! (`\section` = level 1).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn book_section_is_below_chapter() {
    let t = typ("\\documentclass{book}\\begin{document}\\chapter{Ch}\\section{Sec}\\subsection{Sub}body\\end{document}");
    assert!(t.contains("= Ch"), "chapter is level 1; got:\n{t}");
    assert!(t.contains("== Sec"), "section is level 2 under chapter; got:\n{t}");
    assert!(t.contains("=== Sub"), "subsection is level 3; got:\n{t}");
}

#[test]
fn report_starred_section_is_level_2() {
    // The thesis case: `\section*` inside a chapter must be level 2, not level 1.
    let t = typ("\\documentclass{report}\\begin{document}\\chapter{Ch}\\section*{Sec}body\\end{document}");
    assert!(t.contains("level: 2"), "starred section is level 2 in report; got:\n{t}");
}

#[test]
fn article_section_stays_level_1() {
    // Regression: article (no chapters) keeps \section at level 1.
    let t = typ("\\documentclass{article}\\begin{document}\\section{S}\\subsection{Sub}body\\end{document}");
    assert!(t.contains("= S"), "article section is level 1; got:\n{t}");
    assert!(t.contains("== Sub"), "article subsection is level 2; got:\n{t}");
    assert!(!t.contains("=== "), "article has no level-3 from these; got:\n{t}");
}
