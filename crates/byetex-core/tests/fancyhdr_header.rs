//! `fancyhdr` running heads: `\pagestyle{fancy}` + `\fancyhead[L/C/R]{...}`.
//!
//! Measured on the corpus: 12 papers render a running header in the LaTeX truth
//! and we emit NONE — 8053 characters over their first 12 pages alone. Five of
//! the twelve declare it detectably; three do so through `fancyhdr`, which is
//! also what most non-arXiv reports and theses use.
//!
//! Conservative by construction: a slot whose content cannot be rendered as
//! plain text is SKIPPED rather than emitted, because a header is repeated on
//! every page and leaking raw LaTeX there would be worse than having none.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn doc(preamble: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\nBody.\n\\end{{document}}")
}

fn header_line(t: &str) -> String {
    t.lines()
        .find(|l| l.contains("header:"))
        .unwrap_or("<no header>")
        .to_string()
}

#[test]
fn a_centered_fancyhead_becomes_a_page_header() {
    let h = header_line(&typ(&doc(
        "\\usepackage{fancyhdr}\n\\pagestyle{fancy}\n\\fancyhead[C]{A Running Title}",
    )));
    assert!(h.contains("A Running Title"), "header text must reach the page; got: {h}");
}

#[test]
fn left_centre_and_right_slots_are_all_placed() {
    let h = header_line(&typ(&doc(
        "\\pagestyle{fancy}\n\\fancyhead[L]{May 2026}\n\\fancyhead[C]{Middle}\n\\fancyhead[R]{L. Denis et al.}",
    )));
    for want in ["May 2026", "Middle", "L. Denis et al."] {
        assert!(h.contains(want), "slot {want:?} must appear; got: {h}");
    }
}

#[test]
fn fancyhf_resets_previously_declared_slots() {
    // `\fancyhf{}` clears every field; a header declared BEFORE it is gone.
    let h = header_line(&typ(&doc(
        "\\pagestyle{fancy}\n\\fancyhead[C]{Stale}\n\\fancyhf{}\n\\fancyhead[C]{Fresh}",
    )));
    assert!(h.contains("Fresh"), "the later declaration wins; got: {h}");
    assert!(!h.contains("Stale"), "\\fancyhf{{}} must clear earlier slots; got: {h}");
}

#[test]
fn a_slot_that_cannot_be_rendered_is_skipped() {
    // `\@icmltitlerunning` is a class-internal macro. Emitting it raw would put
    // LaTeX on every page, so the slot is dropped instead.
    let t = typ(&doc(
        "\\pagestyle{fancy}\n\\fancyhead[C]{\\small\\bf\\@icmltitlerunning}",
    ));
    assert!(
        !t.contains("icmltitlerunning"),
        "an unrenderable slot must not leak into the header; got:\n{t}"
    );
}

#[test]
fn no_fancyhdr_means_no_header() {
    // The control: documents that never ask for a running head must be unchanged.
    let t = typ(&doc(""));
    assert!(!t.contains("header:"), "must not add a header unbidden; got:\n{t}");
}

#[test]
fn fancyhead_without_pagestyle_fancy_is_still_honoured() {
    // Declaring the fields is the intent; some classes set `\pagestyle{fancy}`
    // themselves in a file we may not have.
    let h = header_line(&typ(&doc("\\fancyhead[C]{Only Fields}")));
    assert!(h.contains("Only Fields"), "fields alone are enough; got: {h}");
}

#[test]
fn a_non_ascii_document_does_not_panic() {
    // The scanner used to walk bytes and slice `&src[i..]`, which panics when the
    // index lands mid-character. A paper with an accented author name in its
    // running head is exactly where this code runs (corpus 2605.31009 — the
    // panic took down the whole conversion, and a sweep that discarded stderr
    // reported it as "no header" rather than as a crash).
    let h = header_line(&typ(&doc(
        "\\pagestyle{fancy}\n\\fancyhead[L]{Café Écoute}\n\\fancyhead[R]{Ångström}\n% naïve\n",
    )));
    assert!(h.contains("Café Écoute"), "non-ASCII slot must survive; got: {h}");
    assert!(h.contains("Ångström"), "non-ASCII slot must survive; got: {h}");
}

#[test]
fn a_commented_slot_body_is_not_lifted_into_the_header() {
    // Corpus 2605.31604 declares `\fancyhead[R]{% Other content ...}`. The comment
    // TEXT was being placed in the header of a paper whose LaTeX truth has no
    // running head at all — a fabricated header on every page.
    let t = typ(&doc(
        "\\pagestyle{fancy}\n\\fancyhead[R]{% Other content for the header, if any\n}\n",
    ));
    assert!(
        !t.contains("Other content"),
        "a comment must not become header text; got:\n{t}"
    );
    assert!(
        !t.contains("header:"),
        "with no renderable slot there must be no header at all; got:\n{t}"
    );
}
