//! `\textcolor{color}{content}` used to drop the colour entirely (corpus: 23
//! papers). Typst has `text(fill: …)`, so the colour now survives in TEXT mode.
//! Resolution order: a `\definecolor`-harvested custom name, then a built-in
//! xcolor name, then an inline `[model]{spec}` form; an unresolvable colour
//! falls back to plain content (never breaks compilation). Math-mode `\textcolor`
//! still renders the content only (colouring math is deferred).

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

fn typ(src: &str) -> String {
    convert_str(src).typst
}

#[test]
fn textcolor_in_text_emits_content() {
    let out = convert_str(r"\textcolor{red}{hello}");
    assert!(out.typst.contains("hello"), "content preserved, got: {}", out.typst);
}

#[test]
fn textcolor_never_leaks_raw_command() {
    let out = convert_str(r"\textcolor{red}{hello}");
    assert!(!out.typst.contains("textcolor"), "raw \\textcolor must not leak, got: {}", out.typst);
}

#[test]
fn textcolor_in_math_emits_content() {
    let out = convert_str(r"$\textcolor{green}{\checkmark}$");
    assert!(!out.typst.contains("extcolor"), "no \\textcolor leak into math, got: {}", out.typst);
}

// ── new: the colour is now applied as `text(fill: …)` in text mode ──────────

#[test]
fn named_color_applies_fill() {
    let t = typ(r"\textcolor{red}{hello}");
    assert!(t.contains("#text(fill: red)[hello]"), "got:\n{t}");
}

#[test]
fn cyan_aliases_to_aqua() {
    // Typst has no `cyan`; aqua is the exact equivalent (#00FFFF).
    let t = typ(r"\textcolor{cyan}{z}");
    assert!(t.contains("#text(fill: aqua)[z]"), "got:\n{t}");
}

#[test]
fn definecolor_html_resolves() {
    // HTML hex → DECIMAL rgb (no `#`, which a table-cell escape pass would
    // mangle to `\#`). FF8800 = 255,136,0.
    let t = typ(r"\definecolor{brand}{HTML}{FF8800}\textcolor{brand}{x}");
    assert!(t.contains("rgb(255, 136, 0)"), "custom HTML color must resolve; got:\n{t}");
    assert!(t.contains("[x]"), "content preserved; got:\n{t}");
}

#[test]
fn inline_rgb_model() {
    let t = typ(r"\textcolor[rgb]{1,0,0}{y}");
    assert!(t.contains("#text(fill: rgb(") && t.contains("[y]"), "got:\n{t}");
}

#[test]
fn unknown_color_falls_back_to_plain_content() {
    let t = typ(r"\textcolor{nosuchcolor}{keep}");
    assert!(t.contains("keep"), "content must survive; got:\n{t}");
    assert!(!t.contains("#text(fill:"), "no fill for an unresolvable color; got:\n{t}");
}
