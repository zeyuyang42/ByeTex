//! Theorem-like environments rendered with NO visible head — `#figure(kind:
//! "theorem", supplement: [Theorem], [body])` shows only the body (the
//! "Theorem N." label appeared only in cross-refs), and the optional
//! `\begin{theorem}[Note]` was dropped entirely (corpus: 20 papers).
//!
//! Fix: the `[Note]` becomes the figure `caption`, and a per-kind show rule
//! renders the head "Theorem N (Note). body".

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn theorem_note_becomes_caption() {
    let t = typ(r"\newtheorem{theorem}{Theorem}\begin{theorem}[Pythagoras]Body.\end{theorem}");
    assert!(
        t.contains("caption: [Pythagoras]"),
        "note not captured; got:\n{t}"
    );
    assert!(t.contains("kind: \"theorem\""), "got:\n{t}");
}

#[test]
fn theorem_kind_gets_head_show_rule() {
    let t = typ(r"\newtheorem{theorem}{Theorem}\begin{theorem}Body.\end{theorem}");
    assert!(
        t.contains("#show figure.where(kind: \"theorem\")"),
        "head show-rule missing; got:\n{t}"
    );
    assert!(
        t.contains("it.supplement") && t.contains("it.body"),
        "got:\n{t}"
    );
}

#[test]
fn theorem_without_note_has_no_caption() {
    let t = typ(r"\newtheorem{theorem}{Theorem}\begin{theorem}Body.\end{theorem}");
    assert!(!t.contains("caption:"), "no note → no caption; got:\n{t}");
}
