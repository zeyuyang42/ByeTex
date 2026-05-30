//! Regression tests for `\subsection{Title}%\n\label{key}` — where a LaTeX
//! `%` comment immediately follows the section command on the same line and
//! the `\label` appears on the next line.
//!
//! In LaTeX, `%` comments out the newline, making the label logically
//! attached to the heading. ByeTex's sibling-label scanner was stopping at
//! `%` rather than skipping through it to find `\label{...}` on the next
//! line, which caused the label to float free as "text" and Typst to abort
//! with "cannot reference text".

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// `\subsection{Title}%` followed by `\label{key}` on the next line:
/// the scanner must skip the `%`-comment and attach the label to the heading.
/// Correct output: `== Title <key>` (label on same line as heading, no blank line between).
#[test]
fn pct_comment_between_section_and_label() {
    let src = "\\begin{document}\n\\subsection{Algebraic formulation}%\n\\label{sub:algebraic}\nBody text.\n\\end{document}";
    let t = typst(src);

    // Both heading and label must be present.
    assert!(
        t.contains("== Algebraic formulation"),
        "heading missing;\noutput:\n{t}"
    );
    assert!(
        t.contains("<sub:algebraic>"),
        "label missing from output;\noutput:\n{t}"
    );
    // The label must appear on the SAME LINE as the heading (no intervening blank line).
    // A blank line between heading and label causes "cannot reference text" in Typst.
    assert!(
        t.contains("== Algebraic formulation <sub:algebraic>"),
        "label not on same line as heading — % comment not skipped;\noutput:\n{t}"
    );
}

/// `\section{Title}%  trailing comment  \n\label{key}`:
/// handles whitespace before `%` and `%` with surrounding spaces.
#[test]
fn pct_comment_with_spaces_before_label() {
    let src = "\\begin{document}\n\\section{Introduction}  %  some comment\n\\label{sec:intro}\nIntro text.\n\\end{document}";
    let t = typst(src);

    assert!(
        t.contains("= Introduction"),
        "heading missing;\noutput:\n{t}"
    );
    assert!(t.contains("<sec:intro>"), "label missing;\noutput:\n{t}");
    assert!(
        t.contains("= Introduction <sec:intro>"),
        "label not on same line as heading — spaces+% comment not skipped;\noutput:\n{t}"
    );
}

/// Without a `%` comment (plain newline), the label must still be attached.
/// This is the existing working case — must not regress.
#[test]
fn plain_newline_between_section_and_label_unchanged() {
    let src = "\\begin{document}\n\\subsection{Results}\n\\label{sub:results}\nResult text.\n\\end{document}";
    let t = typst(src);

    assert!(t.contains("== Results"), "heading missing;\noutput:\n{t}");
    assert!(t.contains("<sub:results>"), "label missing;\noutput:\n{t}");
    assert!(
        t.contains("== Results <sub:results>"),
        "plain-newline regression — label not on same line as heading;\noutput:\n{t}"
    );
}
