//! Beamer `\section` between frames. With the touying emitter (Phase 3a) a `\section`
//! becomes a level-1 heading. Phase 3b gates the section-DIVIDER slide on the deck having
//! `\AtBeginSection` / `\setbeamertemplate{section page}` (real beamer only shows a
//! section-title slide then): present → `= X` divider; absent → `= X <touying:hidden>`
//! (navigation-only, no standalone slide), matching beamer and dropping the spurious pages.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

// No `\AtBeginSection` → un-gated deck → sections are hidden (no divider slide).
const DECK: &str = "\\documentclass{beamer}\\begin{document}\\begin{frame}{F0}intro\\end{frame}\\section{Motivation}\\begin{frame}{F1}x\\end{frame}\\end{document}";

#[test]
fn ungated_section_is_hidden_not_a_divider() {
    let t = typ(DECK);
    // A `\section` in a deck WITHOUT `\AtBeginSection` → `= X <touying:hidden>`: touying
    // keeps it in the heading tree but renders no divider slide.
    assert!(
        t.contains("= Motivation <touying:hidden>"),
        "un-gated section → hidden heading (no divider slide); got:\n{t}"
    );
    // The frame title under it is a level-2 (`==`) slide, distinct from the section.
    assert!(t.contains("== F1"), "frame under the section is a `==` slide; got:\n{t}");
    assert!(!t.contains("pagebreak"), "touying owns slide breaks, not #pagebreak; got:\n{t}");
}

#[test]
fn gated_section_is_a_visible_divider() {
    // A deck WITH `\AtBeginSection` (or `\setbeamertemplate{section page}`) installs a
    // section-title slide, so the `\section` stays a visible `= X` divider (no hidden tag).
    let t = typ("\\documentclass{beamer}\\AtBeginSection[]{\\begin{frame}\\sectionpage\\end{frame}}\\begin{document}\\begin{frame}{F0}intro\\end{frame}\\section{Motivation}\\begin{frame}{F1}x\\end{frame}\\end{document}");
    assert!(
        t.contains("= Motivation") && !t.contains("Motivation <touying:hidden>"),
        "gated section stays a visible `= X` divider; got:\n{t}"
    );
}

#[test]
fn setbeamertemplate_section_page_gates_a_visible_divider() {
    // `\setbeamertemplate{section page}{…}` is the other installer beamer recognises.
    let t = typ("\\documentclass{beamer}\\setbeamertemplate{section page}{x}\\begin{document}\\section{Motivation}\\begin{frame}{F1}x\\end{frame}\\end{document}");
    assert!(
        t.contains("= Motivation") && !t.contains("Motivation <touying:hidden>"),
        "section-page template → visible divider; got:\n{t}"
    );
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
fn non_beamer_section_has_no_pagebreak_or_hidden_tag() {
    let t = typ("\\documentclass{article}\\begin{document}body\\section{Intro}x\\end{document}");
    assert!(t.contains("Intro"), "section rendered");
    assert!(!t.contains("pagebreak"), "paper sections don't pagebreak; got:\n{t}");
    assert!(!t.contains("touying:hidden"), "non-beamer sections are never hidden; got:\n{t}");
}
