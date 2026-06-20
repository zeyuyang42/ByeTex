//! Beamer `\section` between frames (round-4 B6): a section starts its own slide (a
//! section page) rather than its heading bleeding onto the previous frame's slide.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = "\\documentclass{beamer}\\begin{document}\\begin{frame}{F0}intro\\end{frame}\\section{Motivation}\\begin{frame}{F1}x\\end{frame}\\end{document}";

#[test]
fn beamer_section_starts_a_new_slide() {
    let t = typ(DECK);
    assert!(t.contains("Motivation"), "section heading rendered");
    // Between the previous frame's body ("intro") and the section heading there must be
    // a pagebreak, so the section lands on its own slide rather than gluing onto F0.
    let between = t
        .split("intro")
        .nth(1)
        .and_then(|s| s.split("Motivation").next())
        .unwrap_or("");
    assert!(
        between.contains("pagebreak"),
        "a pagebreak must separate the previous frame from the section; got:\n{between}"
    );
}

#[test]
fn non_beamer_section_has_no_pagebreak() {
    let t = typ("\\documentclass{article}\\begin{document}body\\section{Intro}x\\end{document}");
    assert!(t.contains("Intro"), "section rendered");
    assert!(!t.contains("pagebreak"), "paper sections don't pagebreak; got:\n{t}");
}
