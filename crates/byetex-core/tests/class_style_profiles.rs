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

// ─── Abstract styles (Unit 2) ────────────────────────────────────────────────

/// The portion of the output AFTER the title block — i.e. from the `Abstract`
/// heading onward. Used to assert the article \small wrapper sits in the
/// abstract region (not the title block).
fn after_title(t: &str) -> &str {
    let idx = t.find("Abstract").unwrap_or(0);
    &t[idx.saturating_sub(120)..]
}

#[test]
fn article_abstract_is_small_wrapped_centered_bold() {
    let t = typ(include_str!("../../../tests/fixtures/classes/article.tex"));
    let region = after_title(&t);
    assert!(
        region.contains("#text(size: 0.9em)["),
        "article abstract env is \\small (0.9em wrapper); got region:\n{region}\nfull:\n{t}"
    );
    assert!(
        region.contains("#text(weight: \"bold\")[Abstract]"),
        "article abstract heading is centered bold; got:\n{t}"
    );
    assert!(
        region.contains("#pad(x: 2.5em)["),
        "article abstract uses 2.5em quotation pad; got:\n{t}"
    );
}

#[test]
fn neurips_abstract_is_fullwidth_large_bold() {
    let t = typ(include_str!("../../../tests/fixtures/classes/neurips.tex"));
    assert!(
        !t.contains(", columns: 2)"),
        "neurips is single-column — no page columns; got:\n{t}"
    );
    assert!(
        t.contains("#text(size: 1.2em, weight: \"bold\")[Abstract]"),
        "neurips abstract heading is \\large bold; got:\n{t}"
    );
}

#[test]
fn iclr_abstract_is_large_smallcaps() {
    let t = typ(include_str!("../../../tests/fixtures/classes/iclr.tex"));
    assert!(
        t.contains("#text(size: 1.2em)[#smallcaps[Abstract]]"),
        "iclr abstract heading is \\large small-caps; got:\n{t}"
    );
}

#[test]
fn icml_abstract_is_inside_columns() {
    let t = typ(include_str!("../../../tests/fixtures/classes/icml.tex"));
    assert!(t.contains(", columns: 2)"), "icml is two-column (page columns); got:\n{t}");
    let span = pos(&t, "#place(top + center, scope: \"parent\", float: true)[");
    let heading = pos(&t, "#text(size: 1.2em, weight: \"bold\")[Abstract]");
    assert!(
        span < heading && heading != usize::MAX,
        "icml abstract must flow after the spanning title float (in-column); got:\n{t}"
    );
}

#[test]
fn ieee_abstract_run_in_inside_columns_and_deferred_keywords() {
    let t = typ(include_str!(
        "../../../tests/fixtures/classes/ieee_conference.tex"
    ));
    assert!(t.contains(", columns: 2)"), "IEEE is two-column (page columns); got:\n{t}");
    let span = pos(&t, "#place(top + center, scope: \"parent\", float: true)[");
    // The literal `---` from render_abstract_block is post-processed to an
    // em-dash (`—`) by the whole-output typographic pass — faithful to IEEE.
    let runin = pos(&t, "#text(size: 0.9em, weight: \"bold\")[#emph[Abstract]—");
    assert!(
        span < runin && runin != usize::MAX,
        "IEEE run-in abstract must flow after the spanning title float (in-column); got:\n{t}"
    );
    // Deferred IEEEkeywords flow after the abstract (in-column).
    let kws = pos(&t, "*Keywords:* alpha, beta");
    assert!(
        runin < kws && kws != usize::MAX,
        "IEEE keywords must render after the run-in abstract, in-column; got:\n{t}"
    );
}

#[test]
fn acmart_abstract_is_inside_columns() {
    let t = typ(include_str!(
        "../../../tests/fixtures/classes/acmart_sigconf.tex"
    ));
    assert!(t.contains(", columns: 2)"), "acmart sigconf is two-column (page columns); got:\n{t}");
    let span = pos(&t, "#place(top + center, scope: \"parent\", float: true)[");
    let heading = pos(&t, "#text(size: 1.2em, weight: \"bold\")[Abstract]");
    assert!(
        span < heading && heading != usize::MAX,
        "acmart abstract must flow after the spanning title float (in-column); got:\n{t}"
    );
}

#[test]
fn llncs_abstract_is_fullwidth_run_in_bold() {
    let t = typ(include_str!("../../../tests/fixtures/classes/llncs.tex"));
    assert!(
        !t.contains(", columns: 2)"),
        "llncs is single-column; got:\n{t}"
    );
    assert!(
        t.contains("#pad(x: 1cm)[#text(size: 0.9em)[*Abstract.* "),
        "llncs run-in bold abstract with 1cm pad; got:\n{t}"
    );
}

#[test]
fn unknown_class_keeps_neutral_abstract_block() {
    let src = "\\documentclass{amsart}\n\\title{A Study of Conversion Fidelity}\n\\author{Alice Example}\n\\begin{document}\n\\maketitle\n\\begin{abstract}\nA neutral abstract.\n\\end{abstract}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        t.contains("#align(center)[#text(weight: \"bold\")[Abstract]]"),
        "Unknown class abstract heading must stay neutral; got:\n{t}"
    );
    assert!(
        t.contains("#pad(x: 2em)["),
        "Unknown class abstract must keep the neutral 2em pad; got:\n{t}"
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

// ─── Heading sizes (backlog #3) ────────────────────────────────────────────────

#[test]
fn icml_uses_compact_heading_sizes() {
    // ICML sections with \large\bf/\normalsize\bf at a 10pt body → 1.2/1.0/1.0em,
    // not article's inflated 1.44/1.2/1.0.
    let t = typ(include_str!("../../../tests/fixtures/classes/icml.tex"));
    assert!(
        t.contains("#show heading.where(level: 1): set text(size: 1.2em, weight: \"bold\")"),
        "ICML level-1 heading must be 1.2em; got:\n{t}"
    );
    assert!(
        t.contains("#show heading.where(level: 2): set text(size: 1.0em, weight: \"bold\")"),
        "ICML level-2 heading must be 1.0em; got:\n{t}"
    );
}

#[test]
fn article_keeps_large_heading_sizes() {
    // article (and every unprofiled class) keeps the historical 1.44/1.2/1em.
    let t = typ(include_str!("../../../tests/fixtures/classes/article.tex"));
    assert!(
        t.contains("#show heading.where(level: 1): set text(size: 1.44em, weight: \"bold\")"),
        "article level-1 heading must stay 1.44em; got:\n{t}"
    );
    assert!(
        t.contains("#show heading.where(level: 3): set text(size: 1em, weight: \"bold\")"),
        "article level-3 heading must stay the historical 1em literal; got:\n{t}"
    );
}
