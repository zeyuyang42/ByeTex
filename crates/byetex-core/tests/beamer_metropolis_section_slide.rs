//! Metropolis decks show a section-divider SLIDE at each `\section` (the theme
//! installs `\AtBeginSection` internally). ByeTex tagged level-1 sections
//! `<touying:hidden>` unless the deck *explicitly* had `\AtBeginSection` /
//! `\setbeamertemplate{section page}`, so metropolis decks lost their divider
//! slides (truth had them, typst didn't). Detect `\usetheme{metropolis}` and
//! keep the divider. Found by the visual grader on gh-klb2-beamer.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn metropolis_section_renders_divider_not_hidden() {
    let src = r"\documentclass{beamer}\usetheme{metropolis}\begin{document}\section{Math}\begin{frame}{F}Body.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("= Math"), "section heading missing; got:\n{t}");
    assert!(
        !t.contains("= Math <touying:hidden>"),
        "metropolis section was hidden (no divider slide); got:\n{t}"
    );
}

#[test]
fn metropolis_with_options_detected() {
    let src = r"\documentclass{beamer}\usetheme[progressbar=frametitle]{metropolis}\begin{document}\section{Math}\begin{frame}{F}Body.\end{frame}\end{document}";
    let t = typ(src);
    assert!(!t.contains("<touying:hidden>"), "optioned metropolis not detected; got:\n{t}");
}

#[test]
fn non_metropolis_section_still_hidden() {
    // No theme (or a navigation theme) installs no section page → stay hidden.
    let src = r"\documentclass{beamer}\begin{document}\section{Math}\begin{frame}{F}Body.\end{frame}\end{document}";
    let t = typ(src);
    assert!(
        t.contains("= Math <touying:hidden>"),
        "non-metropolis section should stay hidden; got:\n{t}"
    );
}
