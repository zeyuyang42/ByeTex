//! Beamer → touying slides (Phase 3a): a beamer deck now emits a Typst `touying`
//! presentation (metropolis theme) — import + theme `#show:` with `config-info`,
//! `#title-slide()`, frames as `==` slides, and `\section` as `=` section dividers —
//! rather than the old plain-Typst `#set page(paper: "presentation-…")` slides.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = r#"\documentclass[aspectratio=169]{beamer}
\title{Scaling Laws}
\subtitle{An Empirical Study}
\author{Jane Researcher \and Sam Colleague}
\institute{Example University}
\date{June 2026}
\begin{document}
\frame{\titlepage}
\begin{frame}{Outline}
\tableofcontents
\end{frame}
\section{Motivation}
\begin{frame}{Why Scaling Laws?}
Compute budgets are growing fast.
\end{frame}
\end{document}"#;

#[test]
fn imports_touying_and_metropolis_theme() {
    let t = typ(DECK);
    assert!(
        t.contains("#import \"@preview/touying:0.7.3\": *"),
        "deck imports the pinned touying package; got:\n{t}"
    );
    assert!(
        t.contains("#import themes.metropolis: *"),
        "deck imports the metropolis theme; got:\n{t}"
    );
}

#[test]
fn theme_show_carries_config_info_and_aspect() {
    let t = typ(DECK);
    assert!(
        t.contains("#show: metropolis-theme.with("),
        "metropolis-theme applied via #show; got:\n{t}"
    );
    assert!(
        t.contains("aspect-ratio: \"16-9\""),
        "aspectratio=169 → 16-9; got:\n{t}"
    );
    assert!(t.contains("config-info("), "config-info present; got:\n{t}");
    assert!(t.contains("title: [Scaling Laws]"), "title in config-info; got:\n{t}");
    assert!(
        t.contains("subtitle: [An Empirical Study]"),
        "subtitle in config-info; got:\n{t}"
    );
    assert!(
        t.contains("Jane Researcher") && t.contains("Sam Colleague"),
        "authors in config-info; got:\n{t}"
    );
    assert!(
        t.contains("institution: [Example University]"),
        "institute → institution; got:\n{t}"
    );
    assert!(t.contains("date: [June 2026]"), "date in config-info; got:\n{t}");
}

#[test]
fn titlepage_emits_title_slide_call() {
    let t = typ(DECK);
    assert!(
        t.contains("#title-slide()"),
        "\\frame{{\\titlepage}} → #title-slide(); got:\n{t}"
    );
    // The old centered hand-rolled title block must be gone.
    assert!(
        !t.contains("#text(size: 1.5em, weight: \"bold\")[Scaling Laws]"),
        "old hand-rolled title block must not be emitted; got:\n{t}"
    );
}

#[test]
fn frame_is_a_level2_slide_heading() {
    let t = typ(DECK);
    assert!(
        t.contains("== Why Scaling Laws?"),
        "frame title → `== Title` touying slide; got:\n{t}"
    );
    // Old bold #text frame title is gone.
    assert!(
        !t.contains("#text(size: 1.2em, weight: \"bold\""),
        "old #text frame title must not be emitted; got:\n{t}"
    );
    assert!(
        !t.contains("#pagebreak"),
        "touying slides use `==`, not #pagebreak; got:\n{t}"
    );
}

#[test]
fn section_is_a_level1_divider() {
    let t = typ(DECK);
    assert!(t.contains("= Motivation"), "section → `= X` divider; got:\n{t}");
}

#[test]
fn aspect_ratio_4_3_default() {
    let t = typ("\\documentclass{beamer}\\title{T}\\begin{document}\\begin{frame}{S}x\\end{frame}\\end{document}");
    assert!(
        t.contains("aspect-ratio: \"4-3\""),
        "default beamer → 4-3; got:\n{t}"
    );
}

#[test]
fn no_neutral_preamble_page_set_for_beamer() {
    let t = typ(DECK);
    assert!(
        !t.contains("presentation-16-9\""),
        "no plain `#set page(paper: \"presentation-…\")`; touying owns the page; got:\n{t}"
    );
    assert!(
        !t.contains("#set heading(numbering"),
        "no #set heading(numbering) — touying numbers slides itself; got:\n{t}"
    );
}
