//! Bibliography font size, detected from the document's own definitions.
//!
//! Reference lists are set smaller than body text by most venue classes, and the
//! references section is a large, dense block of text — on 2605.31604 the last
//! three pages of the LaTeX truth are 100% small-tier while ours are 0%.
//!
//! Measured across the corpus by comparing the small-tier share of the LAST
//! pages of truth vs ours: 19 papers show a large gap, several at 1.00 vs 0.00.
//! That is more than twice the reach of the caption fix (#517), over a bigger
//! mass of text — it was the single largest remaining contributor to
//! `layout_small_tier_share_delta`.
//!
//! Two detectable signals, in priority order:
//!   1. `\def\bibfont{\small}` / `\renewcommand{\bibfont}{...}` — natbib's hook
//!   2. a size switch inside a bundled class's `thebibliography` definition

use byetex_core::{convert, ConvertOptions};

fn conv(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn rule(typst: &str) -> String {
    typst
        .lines()
        .find(|l| l.contains("show bibliography: set text"))
        .unwrap_or("<no bibliography rule>")
        .to_string()
}

fn doc(preamble: &str) -> String {
    format!(
        "\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n\
         Text \\cite{{k}}.\n\\bibliography{{refs}}\n\\end{{document}}"
    )
}

#[test]
fn def_bibfont_small_sizes_the_bibliography() {
    let r = rule(&conv(&doc("\\def\\bibfont{\\small}")));
    assert!(r.contains("size: 0.9em"), "\\bibfont{{\\small}} must size it; got: {r}");
}

#[test]
fn renewcommand_bibfont_is_read_too() {
    let r = rule(&conv(&doc("\\renewcommand{\\bibfont}{\\footnotesize}")));
    assert!(r.contains("size: 0.8em"), "\\renewcommand form; got: {r}");
}

#[test]
fn a_size_in_a_thebibliography_definition_is_read() {
    let r = rule(&conv(&doc(
        "\\renewenvironment{thebibliography}[1]{\\section*{References}\\footnotesize\\begin{list}{}{}}{\\end{list}}",
    )));
    assert!(r.contains("size: 0.8em"), "thebibliography definition; got: {r}");
}

#[test]
fn bibfont_wins_over_a_thebibliography_definition() {
    // natbib's hook is the more specific, later-applied statement.
    let r = rule(&conv(&doc(
        "\\renewenvironment{thebibliography}[1]{\\footnotesize}{}\n\\def\\bibfont{\\small}",
    )));
    assert!(r.contains("size: 0.9em"), "\\bibfont wins; got: {r}");
}

#[test]
fn a_normalsize_bibfont_emits_no_rule() {
    let r = rule(&conv(&doc("\\def\\bibfont{\\normalsize}")));
    assert_eq!(r, "<no bibliography rule>", "normalsize needs no rule; got: {r}");
}

#[test]
fn a_commented_bibfont_is_ignored() {
    let r = rule(&conv(&doc("%\\def\\bibfont{\\small}")));
    assert_eq!(r, "<no bibliography rule>", "commented out; got: {r}");
}

#[test]
fn no_declaration_leaves_output_unchanged() {
    // The control: nothing declared, no rule, no change for most documents.
    let r = rule(&conv(&doc("")));
    assert_eq!(r, "<no bibliography rule>", "must not size unbidden; got: {r}");
}

#[test]
fn a_size_elsewhere_in_the_preamble_is_not_mistaken_for_it() {
    // A `\small` that has nothing to do with the bibliography must not fire.
    let r = rule(&conv(&doc("\\newcommand{\\note}[1]{{\\small #1}}")));
    assert_eq!(r, "<no bibliography rule>", "unrelated \\small; got: {r}");
}
