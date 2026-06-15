//! Task 2c (layout fidelity): page margins / paper from the `geometry` package.
//!
//! `\usepackage[...]{geometry}` options and `\geometry{...}` command keys are
//! merged into `#set page(margin: ...)` (and `paper:` for a paper-size flag).
//! Lengths pass through to Typst's compatible units (in/cm/mm/pt/em). When no
//! geometry is present the neutral default margin is kept.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn doc(preamble: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\nBody.\n\\end{{document}}")
}

#[test]
fn uniform_margin_from_package() {
    let t = typ(&doc("\\usepackage[margin=1.5in]{geometry}"));
    assert!(
        t.contains("margin: 1.5in"),
        "expected uniform `margin: 1.5in`; got:\n{t}"
    );
}

#[test]
fn individual_margins_from_package() {
    let t = typ(&doc(
        "\\usepackage[top=2cm,bottom=3cm,left=1in,right=1in]{geometry}",
    ));
    assert!(
        t.contains("margin: (top: 2cm, bottom: 3cm, left: 1in, right: 1in)"),
        "expected a per-side margin dict; got:\n{t}"
    );
}

#[test]
fn geometry_command_applies() {
    let t = typ(&doc("\\geometry{margin=20mm}"));
    assert!(
        t.contains("margin: 20mm"),
        "expected `margin: 20mm`; got:\n{t}"
    );
}

#[test]
fn paper_flag_from_geometry() {
    let t = typ(&doc("\\usepackage[a4paper,margin=1in]{geometry}"));
    assert!(t.contains("paper: \"a4\""), "expected a4 paper; got:\n{t}");
    assert!(
        t.contains("margin: 1in"),
        "expected `margin: 1in`; got:\n{t}"
    );
}

#[test]
fn command_keys_merge_over_package() {
    // `\geometry{top=2cm}` overrides only `top`; the package's `margin=1in`
    // fills the other three sides.
    let t = typ(&doc(
        "\\usepackage[margin=1in]{geometry}\n\\geometry{top=2cm}",
    ));
    assert!(
        t.contains("margin: (top: 2cm, bottom: 1in, left: 1in, right: 1in)"),
        "expected command key merged over package margin; got:\n{t}"
    );
}

#[test]
fn hmargin_vmargin_expand_to_sides() {
    let t = typ(&doc("\\usepackage[hmargin=2cm,vmargin=3cm]{geometry}"));
    assert!(
        t.contains("margin: (top: 3cm, bottom: 3cm, left: 2cm, right: 2cm)"),
        "expected hmargin/vmargin expanded; got:\n{t}"
    );
}

#[test]
fn default_margin_kept_without_geometry() {
    let t = typ(&doc(""));
    assert!(
        t.contains("margin: (x: 1in, y: 1in)"),
        "expected the neutral default margin; got:\n{t}"
    );
}

#[test]
fn unsupported_length_is_skipped_not_emitted() {
    // A relative length we can't translate (e.g. `0.8\textwidth`) is ignored,
    // leaving the neutral default rather than emitting a broken value.
    let t = typ(&doc("\\usepackage[margin=0.8\\textwidth]{geometry}"));
    assert!(
        t.contains("margin: (x: 1in, y: 1in)"),
        "unsupported length should fall back to default; got:\n{t}"
    );
    assert!(
        !t.contains("textwidth"),
        "raw LaTeX length must not leak into output; got:\n{t}"
    );
}
