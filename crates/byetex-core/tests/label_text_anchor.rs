//! Regression: a `\label` that lands on plain text or a list item (not a
//! heading / figure / equation) must emit a *referenceable* anchor, not a bare
//! `<key>` glued onto text.
//!
//! Typst's `@key` aborts with "cannot reference text" when `<key>` is attached
//! to a paragraph / list item, and aborts with "label does not exist" when the
//! enclosing environment was dropped entirely. arXiv:2605.22724 (labels on
//! `enumerate` items inside an `assumptions` theorem env) and 2605.22765 (a
//! `\label` inside a custom `\newenvironment`) both hit this.
//!
//! Fix: the inline `\label` handler emits a hidden, self-numbered figure
//! (`kind: "anchor"`) which IS referenceable, plus a `#show` rule that hides it.
//! Heading / figure / equation labels keep going through their own structural
//! paths and stay bare.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// A `\label` on an `enumerate` item must become a referenceable anchor.
#[test]
fn label_on_list_item_emits_referenceable_anchor() {
    let src = "\\begin{enumerate}\n\\item First condition. \\label{a:one}\n\
        \\item Second condition.\n\\end{enumerate}\nSee \\ref{a:one}.";
    let t = typst(src);

    assert!(
        t.contains("kind: \"anchor\""),
        "a label on a list item must emit a referenceable anchor figure;\noutput:\n{t}"
    );
    // The anchor must be wrapped in an inline `#box[...]` so a block-level
    // figure does not split the surrounding paragraph / list item.
    assert!(
        t.contains("#box[#figure(kind: \"anchor\""),
        "the anchor figure must be wrapped in an inline #box to avoid a \
         spurious paragraph break;\noutput:\n{t}"
    );
    assert!(
        t.contains("<a:one>"),
        "the anchor must carry the sanitized label key;\noutput:\n{t}"
    );
    assert!(
        t.contains("#show figure.where(kind: \"anchor\")"),
        "the anchor-hiding show rule must be emitted when anchors are used;\noutput:\n{t}"
    );
}

/// An inline `\label` after plain text (mid-paragraph) must also be an anchor.
#[test]
fn inline_text_label_emits_anchor() {
    let src = "This holds by construction. \\label{eq:prop}\nSee \\ref{eq:prop}.";
    let t = typst(src);
    assert!(
        t.contains("kind: \"anchor\"") && t.contains("<eq:prop>"),
        "an inline text label must emit a referenceable anchor;\noutput:\n{t}"
    );
}

/// Non-regression: a `\section` label still attaches to the heading and must
/// NOT be turned into an anchor (heading refs render the section number).
#[test]
fn section_label_stays_bare_not_anchor() {
    let src = "\\section{Introduction}\\label{sec:i}\nSee \\ref{sec:i}.";
    let t = typst(src);
    assert!(
        t.contains("= Introduction <sec:i>"),
        "a section label must attach to the heading;\noutput:\n{t}"
    );
    assert!(
        !t.contains("kind: \"anchor\""),
        "a section label must NOT use an anchor;\noutput:\n{t}"
    );
}

/// A full document (with a `\documentclass`) emits the anchor show rule
/// exactly once — not duplicated by the fragment-preamble fallback path.
#[test]
fn full_document_emits_show_rule_once() {
    let src = "\\documentclass{article}\n\\begin{document}\n\
        Text. \\label{x:one}\nSee \\ref{x:one}.\n\\end{document}";
    let t = typst(src);
    let count = t.matches("#show figure.where(kind: \"anchor\")").count();
    assert_eq!(
        count, 1,
        "the anchor show rule must appear exactly once (got {count});\noutput:\n{t}"
    );
}

/// Non-regression: a document with no text labels emits no anchor show rule.
#[test]
fn no_anchor_show_rule_without_text_labels() {
    let src = "\\section{Plain}\nJust text, no labels here.";
    let t = typst(src);
    assert!(
        !t.contains("kind: \"anchor\""),
        "no anchor machinery should appear when there are no text labels;\noutput:\n{t}"
    );
}
