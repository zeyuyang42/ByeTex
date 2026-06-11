//! Unit 1 — per-class `StyleProfile`: class-faithful title size / weight /
//! small-caps / horizontal rules + body font, derived from the detected
//! `DocClass`. Sizes are em relative to the body font size (the conference
//! classes all run 10pt bodies, so e.g. 17pt == 1.7em).
//!
//! Ground truth (verified against the actual class files):
//! - article.cls \maketitle is {\LARGE \@title} — 1.728em, NOT bold.
//! - neurips_2026.sty: 4pt toptitlebar rule, \LARGE(=17pt) bold title,
//!   1pt bottomtitlebar rule; authors come AFTER the bottom rule.
//! - icml2026.sty: 1pt rules above and below a {\Large\bf}(=14pt) title.
//! - iclr_conference.sty: {\LARGE\sc \@title} — small caps, regular weight.
//! - IEEEtran.cls (non-technote): {\Huge ... \@title} — 2.4em, regular.
//! - acmart: sans bold \LARGE truth; serif bold approximation + the
//!   Libertinus Serif body font (matches acmart's Linux Libertine).
//! - llncs.cls: {\Large \bfseries\boldmath \@title} — 1.44em bold.
//! - elsarticle / Unknown: deliberately unprofiled — byte-identical to the
//!   neutral fallback (1.5em bold, New Computer Modern).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// Byte offset of `needle` in `hay`, or `usize::MAX` if absent (so "appears
/// before" comparisons fail loudly rather than panicking on unwrap).
fn pos(hay: &str, needle: &str) -> usize {
    hay.find(needle).unwrap_or(usize::MAX)
}

/// The line of `hay` that contains `needle` (panics with the full output when
/// absent, so failures show what was actually emitted).
fn line_with<'a>(hay: &'a str, needle: &str) -> &'a str {
    hay.lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line containing {needle:?} in:\n{hay}"))
}

const TITLE: &str = "A Study of Conversion Fidelity";

// ─── article (ArxivArticle) ───────────────────────────────────────────────────

#[test]
fn article_title_is_large_regular_weight() {
    let t = typ(include_str!("../../../tests/fixtures/classes/article.tex"));
    let title_line = line_with(&t, TITLE);
    assert!(
        title_line.contains("#text(size: 1.728em)["),
        "article title must be \\LARGE (1.728em); got line:\n{title_line}\nfull:\n{t}"
    );
    assert!(
        !title_line.contains("weight"),
        "article.cls \\maketitle title is NOT bold; got line:\n{title_line}"
    );
}

// ─── NeurIPS ──────────────────────────────────────────────────────────────────

#[test]
fn neurips_title_rules_and_large_bold() {
    let t = typ(include_str!("../../../tests/fixtures/classes/neurips.tex"));
    let top_rule = pos(&t, "#line(length: 100%, stroke: 4pt)");
    let title = pos(&t, "#text(size: 1.7em, weight: \"bold\")[");
    let bottom_rule = pos(&t, "#line(length: 100%, stroke: 1pt)");
    assert!(
        top_rule < title,
        "4pt toptitlebar rule must precede the title; got:\n{t}"
    );
    assert!(
        title < bottom_rule,
        "1pt bottomtitlebar rule must follow the title; got:\n{t}"
    );
    // Authors come AFTER the bottom rule (matching \@maketitle order). Note:
    // `#set document(author: ...)` is prepended, so use the LAST occurrence.
    let author = t.rfind("Alice Example").unwrap_or(0);
    assert!(
        bottom_rule < author,
        "authors must render below the bottom rule; got:\n{t}"
    );
}

// ─── ICML ─────────────────────────────────────────────────────────────────────

#[test]
fn icml_title_rules_and_large_bold() {
    let t = typ(include_str!("../../../tests/fixtures/classes/icml.tex"));
    let title = pos(&t, "#text(size: 1.4em, weight: \"bold\")[");
    assert!(
        title != usize::MAX,
        "icml title must be \\Large (1.4em) bold; got:\n{t}"
    );
    // 1pt rules both above and below the title.
    let first_rule = pos(&t, "#line(length: 100%, stroke: 1pt)");
    let last_rule = t.rfind("#line(length: 100%, stroke: 1pt)").unwrap_or(0);
    assert!(
        first_rule < title && title < last_rule,
        "icml title must sit between two 1pt rules; got:\n{t}"
    );
}

// ─── ICLR ─────────────────────────────────────────────────────────────────────

#[test]
fn iclr_title_is_large_smallcaps_regular_weight() {
    let t = typ(include_str!("../../../tests/fixtures/classes/iclr.tex"));
    assert!(
        t.contains("#text(size: 1.7em)[#smallcaps["),
        "iclr title must be \\LARGE small-caps, regular weight; got:\n{t}"
    );
}

// ─── IEEE conference ──────────────────────────────────────────────────────────

#[test]
fn ieee_title_is_huge_regular_weight() {
    let t = typ(include_str!(
        "../../../tests/fixtures/classes/ieee_conference.tex"
    ));
    let title_line = line_with(&t, TITLE);
    assert!(
        title_line.contains("#text(size: 2.4em)["),
        "IEEEtran title must be \\Huge (2.4em); got line:\n{title_line}\nfull:\n{t}"
    );
    assert!(
        !title_line.contains("weight"),
        "IEEEtran title is NOT bold; got line:\n{title_line}"
    );
}

// ─── ACM sigconf ──────────────────────────────────────────────────────────────

#[test]
fn acmart_title_large_bold_and_libertinus_body_font() {
    let t = typ(include_str!(
        "../../../tests/fixtures/classes/acmart_sigconf.tex"
    ));
    assert!(
        t.contains("#text(size: 1.728em, weight: \"bold\")["),
        "acmart title must be \\LARGE (1.728em) bold; got:\n{t}"
    );
    assert!(
        t.contains("font: \"Libertinus Serif\""),
        "acmart body font must be Libertinus Serif; got:\n{t}"
    );
}

// ─── LLNCS ────────────────────────────────────────────────────────────────────

#[test]
fn llncs_title_is_large_bold() {
    let t = typ(include_str!("../../../tests/fixtures/classes/llncs.tex"));
    assert!(
        t.contains("#text(size: 1.44em, weight: \"bold\")["),
        "llncs title must be \\Large (1.44em) bold; got:\n{t}"
    );
}

// ─── elsarticle (deliberately unprofiled) ─────────────────────────────────────

#[test]
fn elsarticle_keeps_neutral_title() {
    let t = typ(include_str!(
        "../../../tests/fixtures/classes/elsarticle.tex"
    ));
    let title_line = line_with(&t, TITLE);
    assert!(
        title_line.contains("#text(size: 1.5em, weight: \"bold\")["),
        "elsarticle is unprofiled — title must stay the neutral fallback; got line:\n{title_line}\nfull:\n{t}"
    );
}

// ─── Unknown class (amsart) — neutral() byte-compat guard ────────────────────

#[test]
fn unknown_class_keeps_neutral_title_and_font() {
    let src = "\\documentclass{amsart}\n\\title{A Study of Conversion Fidelity}\n\\author{Alice Example}\n\\begin{document}\n\\maketitle\nBody.\n\\end{document}";
    let t = typ(src);
    let title_line = line_with(&t, TITLE);
    assert!(
        title_line.contains("#text(size: 1.5em, weight: \"bold\")["),
        "Unknown class must keep the neutral title line; got line:\n{title_line}\nfull:\n{t}"
    );
    assert!(
        t.contains("font: \"New Computer Modern\""),
        "Unknown class must keep the neutral body font; got:\n{t}"
    );
}
