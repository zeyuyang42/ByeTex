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

/// A comma in the over-text used to leak straight into the `attach(base, t: …)`
/// arg list, where Typst reads it as a stray SECOND positional argument →
/// `error: unexpected argument` (corpus 2605.31063, regression ~PR #286). The
/// script must be wrapped so the comma is contained.
#[test]
fn overset_with_comma_in_script_is_wrapped() {
    let t = math(r"\overset{a, b}{=}");
    // The comma must NOT sit unguarded inside the attach arg list.
    assert!(
        !t.contains("t: a, b)"),
        "comma must be contained, not a bare second arg; got:\n{t}"
    );
    // The over-text is wrapped in a comma-transparent math box.
    assert!(t.contains("#box[$"), "script wrapped in a math box; got:\n{t}");
    assert!(t.contains("attach(=, t: "), "still an attach; got:\n{t}");
}

#[test]
fn stackrel_with_comma_in_script_is_wrapped() {
    let t = math(r"\stackrel{x, y}{\to}");
    assert!(
        !t.contains("t: x, y)"),
        "comma must be contained; got:\n{t}"
    );
    assert!(t.contains("#box[$"), "script wrapped; got:\n{t}");
}

/// The comma-free single-token case must keep the bare, unwrapped form so plain
/// `\overset{x}{=}` renders identically.
#[test]
fn overset_without_comma_stays_unwrapped() {
    let t = math(r"\overset{x}{=}");
    assert!(t.contains("attach(=, t: x)"), "bare form preserved; got:\n{t}");
    assert!(!t.contains("#box["), "no box for the simple case; got:\n{t}");
}
