//! Beamer `\tableofcontents` (round-4 B-toc): renders a section outline on the slide
//! instead of being dropped. Sections convert to headings, so a Typst `#outline` lists
//! them. Papers keep the previous drop (Typst doesn't model a paper ToC the same way).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = "\\documentclass{beamer}\\begin{document}\\begin{frame}{Outline}\\tableofcontents\\end{frame}\\section{Motivation}\\begin{frame}{F1}x\\end{frame}\\section{Method}\\begin{frame}{F2}y\\end{frame}\\end{document}";

#[test]
fn beamer_tableofcontents_emits_outline() {
    let t = typ(DECK);
    assert!(t.contains("#outline("), "beamer \\tableofcontents → #outline; got:\n{t}");
    // The section headings it lists are still present.
    assert!(t.contains("Motivation") && t.contains("Method"), "sections present");
}

#[test]
fn non_beamer_tableofcontents_still_dropped() {
    let t = typ("\\documentclass{article}\\begin{document}\\tableofcontents\\section{Intro}x\\end{document}");
    assert!(!t.contains("#outline("), "paper \\tableofcontents stays dropped; got:\n{t}");
}
