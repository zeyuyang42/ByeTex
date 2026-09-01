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
fn section_is_a_level1_heading_hidden_when_ungated() {
    let t = typ(DECK);
    // This DECK has no `\AtBeginSection`, so the section is navigation-only: a level-1
    // heading tagged `<touying:hidden>` (no divider slide). See beamer_section_slide.rs
    // for the gated/un-gated split.
    assert!(
        t.contains("= Motivation <touying:hidden>"),
        "un-gated section → hidden level-1 heading; got:\n{t}"
    );
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

// --- Slide geometry -------------------------------------------------------
//
// touying's `aspect-ratio:` only picks a Typst *presentation preset*
// (`presentation-4-3` = 280x210mm, `presentation-16-9` = 297x167mm), which is
// 1.9-2.2x larger in each dimension than the slide beamer actually produces.
// The physical size matters: a `\includegraphics[width=6cm]` fills 47% of a real
// 128mm slide but only 21% of a 280mm one. Each expected size below was measured
// from a tectonic render of `\documentclass[aspectratio=N]{beamer}`.

fn deck_with(opts: &str) -> String {
    typ(&format!(
        "\\documentclass{opts}{{beamer}}\\title{{T}}\\begin{{document}}\
         \\begin{{frame}}{{S}}x\\end{{frame}}\\end{{document}}"
    ))
}

#[test]
fn default_slide_is_beamers_128x96mm() {
    let t = deck_with("");
    assert!(
        t.contains("config-page(width: 128mm, height: 96mm)"),
        "beamer's default slide is 128x96mm, not Typst's 280x210mm preset; got:\n{t}"
    );
}

#[test]
fn aspectratio_169_slide_is_160x90mm() {
    let t = deck_with("[aspectratio=169]");
    assert!(
        t.contains("config-page(width: 160mm, height: 90mm)"),
        "aspectratio=169 is 160x90mm, not Typst's 297x167mm preset; got:\n{t}"
    );
}

#[test]
fn aspectratio_1610_is_16_10_not_16_9() {
    // Regression: 1610 and 149 were both mapped onto `presentation-16-9`, so the
    // deck came out at the wrong *shape*, not merely the wrong size.
    let t = deck_with("[aspectratio=1610]");
    assert!(
        t.contains("config-page(width: 160mm, height: 100mm)"),
        "aspectratio=1610 is 16:10 (160x100mm), not 16:9; got:\n{t}"
    );
    let t = deck_with("[aspectratio=149]");
    assert!(
        t.contains("config-page(width: 140mm, height: 90mm)"),
        "aspectratio=149 is 14:9 (140x90mm), not 16:9; got:\n{t}"
    );
}

#[test]
fn body_font_defaults_to_beamers_11pt() {
    // The theme hard-codes `set text(size: 20pt)` for its 297mm-wide preset. On a
    // real 160mm slide that is ~2x too large, so the deck must restate beamer's
    // own body size; every em-derived length in the theme (margins, header,
    // footer) follows it.
    let t = deck_with("[aspectratio=169]");
    assert!(
        t.contains("#set text(size: 11pt)"),
        "beamer's default body size is 11pt; got:\n{t}"
    );
}

#[test]
fn body_font_honors_the_class_option() {
    let t = deck_with("[10pt]");
    assert!(
        t.contains("#set text(size: 10pt)"),
        "`[10pt]` sets beamer's body size; got:\n{t}"
    );
    // beamer accepts sizes the article classes do not.
    let t = deck_with("[aspectratio=169,14pt]");
    assert!(
        t.contains("#set text(size: 14pt)"),
        "beamer accepts 14pt; got:\n{t}"
    );
}

#[test]
fn aspectratio_2013_is_beamers_140x91mm() {
    // beamer.cls special-cases 20:13 at 140x91mm — it does NOT fall out of the
    // computed rule below (which would give 147.7x96mm).
    let t = deck_with("[aspectratio=2013]");
    assert!(
        t.contains("config-page(width: 140mm, height: 91mm)"),
        "aspectratio=2013 is 140x91mm; got:\n{t}"
    );
}

#[test]
fn unlisted_aspectratio_uses_beamers_computed_size() {
    // Outside its table of eight, beamer fixes the height at 96mm and scales the
    // width by the ratio, splitting the digits down the middle. Every expectation
    // below was measured from a tectonic render, and every one differs from the
    // 4:3 default — a case like `1612` (which computes back to 128x96mm) would
    // pass just as well without the computed path, so it is not used here.
    for (opt, expected) in [
        ("[aspectratio=53]", "config-page(width: 160mm, height: 96mm)"),
        ("[aspectratio=118]", "config-page(width: 132mm, height: 96mm)"),
        ("[aspectratio=1210]", "config-page(width: 115.2mm, height: 96mm)"),
    ] {
        let t = deck_with(opt);
        assert!(t.contains(expected), "{opt} → {expected}; got:\n{t}");
    }
}

#[test]
fn aspectratio_tolerates_spaces_around_equals() {
    // LaTeX key-value class options allow `key = value`; the deck must not
    // silently fall back to 4:3.
    let t = deck_with("[aspectratio = 169]");
    assert!(
        t.contains("config-page(width: 160mm, height: 90mm)"),
        "`aspectratio = 169` (spaced) is still 160x90mm; got:\n{t}"
    );
}

#[test]
fn a_nonsense_aspectratio_falls_back_to_4_3() {
    // Not a digit pair (and `160` would divide by a zero height), so neither the
    // table nor the computed rule applies — take beamer's own default rather than
    // emitting a degenerate page.
    for opt in ["[aspectratio=abc]", "[aspectratio=]", "[aspectratio=160]"] {
        let t = deck_with(opt);
        assert!(
            t.contains("config-page(width: 128mm, height: 96mm)"),
            "{opt} → beamer's 4:3 default; got:\n{t}"
        );
    }
}
