//! Regression test: a section heading must always start on its own line.
//!
//! Typst only parses `==` as a heading at the start of a line. When a heading
//! immediately followed inline content that did not end with a newline — e.g. a
//! `remark`/theorem environment ending in `<rem:foo>`, then `\subsection{…}` —
//! the `==` was glued onto the previous line (`<rem:foo>== Title <sub:bar>`), so
//! it became plain text and the `<sub:bar>` label attached to *text*. Any
//! `@sub:bar` reference then made Typst abort with `cannot reference text`.
//! arXiv:2605.22159 hit this.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// A `\subsection` immediately after a `remark` environment must emit its `==`
/// at the start of a line, with the label on the heading (not glued to text).
#[test]
fn heading_after_inline_label_starts_on_own_line() {
    let src = "\\begin{remark}\\label{rem:c}Body text.\\end{remark}\
        \\subsection{Algebraic Section}\\label{sub:a}\nSee \\ref{sub:a}.";
    let t = typst(src);

    // No heading marker glued onto a preceding label-close.
    assert!(
        !t.contains(">=="),
        "heading `==` must not be glued after a `<label>`;\noutput:\n{t}"
    );
    // The heading marker must sit at the start of a line.
    let h = t
        .find("== Algebraic Section")
        .expect("heading text present");
    assert!(
        t[..h].ends_with('\n'),
        "`== Algebraic Section` must start on its own line;\noutput:\n{t}"
    );
    // The subsection label must be attached to the heading line.
    assert!(
        t.contains("== Algebraic Section <sub:a>"),
        "label must attach to the heading;\noutput:\n{t}"
    );
}

/// No-regression: an ordinary `\section` after plain text still emits a single
/// well-formed heading (the leading paragraph break is idempotent).
#[test]
fn ordinary_section_unaffected() {
    let t = typst("Some intro text.\n\n\\section{Intro}\nBody.");
    assert!(
        t.contains("= Intro"),
        "section heading present;\noutput:\n{t}"
    );
    // Exactly one `= Intro` heading marker (no duplication / mangling).
    assert_eq!(
        t.matches("= Intro").count(),
        1,
        "exactly one heading;\noutput:\n{t}"
    );
}
