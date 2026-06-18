//! The overset family (`\overset`, `\underset`, `\stackrel`, `\accentset`) used to
//! drop BOTH arguments and emit the bare command name as a string in math
//! (`"accentset"`, `"overset"`, …) — dogfood backlog F8 (corpus 2605.31510 had 37
//! `\accentset` sites). They now map to Typst `attach(base, t|b: script)`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn math(inner: &str) -> String {
    typ(&format!(
        "\\documentclass{{article}}\\begin{{document}}${inner}$\\end{{document}}"
    ))
}

#[test]
fn overset_maps_to_attach_top() {
    let t = math(r"\overset{a}{b}");
    assert!(t.contains("attach(b, t: a)"), "got:\n{t}");
    assert!(!t.contains("\"overset\""), "no leaked command name; got:\n{t}");
}

#[test]
fn underset_maps_to_attach_bottom() {
    let t = math(r"\underset{a}{b}");
    assert!(t.contains("attach(b, b: a)"), "got:\n{t}");
}

#[test]
fn stackrel_maps_to_attach_top() {
    let t = math(r"\stackrel{x}{=}");
    assert!(t.contains("attach(=, t: x)"), "got:\n{t}");
    assert!(!t.contains("\"stackrel\""), "no leaked command name; got:\n{t}");
}

#[test]
fn accentset_maps_to_attach_top() {
    let t = math(r"\accentset{\circ}{h}");
    assert!(
        t.contains("attach(h, t: ") && !t.contains("\"accentset\""),
        "accentset must render base h with the accent on top; got:\n{t}"
    );
}

#[test]
fn accentset_with_bold_base_keeps_base() {
    let t = math(r"\accentset{\star}{S}");
    assert!(t.contains("attach(S, t: "), "base S preserved; got:\n{t}");
    assert!(!t.contains("\"accentset\""), "no leak; got:\n{t}");
}
