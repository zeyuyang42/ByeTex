//! Beamer `\section` between frames: a section is its own section-divider slide. With
//! the touying emitter (Phase 3a) a `\section` becomes a level-1 heading (`= X`), which
//! metropolis renders as an automatic section divider slide (no manual `#pagebreak`).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = "\\documentclass{beamer}\\begin{document}\\begin{frame}{F0}intro\\end{frame}\\section{Motivation}\\begin{frame}{F1}x\\end{frame}\\end{document}";

#[test]
fn beamer_section_is_a_level1_divider() {
    let t = typ(DECK);
    // A `\section` → `= X`, which touying renders as a section-divider slide.
    assert!(t.contains("= Motivation"), "section → `= X` divider; got:\n{t}");
    // The frame title under it is a level-2 (`==`) slide, distinct from the divider.
    assert!(t.contains("== F1"), "frame under the section is a `==` slide; got:\n{t}");
    assert!(!t.contains("pagebreak"), "touying owns slide breaks, not #pagebreak; got:\n{t}");
}

#[test]
fn beamer_subsection_is_demoted_below_frame_level() {
    // A `\subsection` (level 2) would collide with frame slides (also `==`); it is
    // demoted to a level-3 heading so it stays inside the current slide.
    let t = typ("\\documentclass{beamer}\\begin{document}\\section{Sec}\\begin{frame}{F}\\subsection{Sub}\nbody\\end{frame}\\end{document}");
    assert!(t.contains("= Sec"), "section is `= X`; got:\n{t}");
    assert!(t.contains("=== Sub"), "subsection demoted to `=== X`; got:\n{t}");
}

#[test]
fn non_beamer_section_has_no_pagebreak() {
    let t = typ("\\documentclass{article}\\begin{document}body\\section{Intro}x\\end{document}");
    assert!(t.contains("Intro"), "section rendered");
    assert!(!t.contains("pagebreak"), "paper sections don't pagebreak; got:\n{t}");
}
