//! Cross-reference/label keys with `_` are normalized before tree-sitter parses
//! them (tree-sitter mis-reads the `_` as a math subscript, which on complex
//! documents cascades into a whole-document parse failure — corpus 2605.22728).
//! These tests are REGRESSION GUARDS for the round-trip: the sentinel substitution
//! must be invisible in the output (`_` restored on both the label-def and ref
//! sides, no control byte leaks), and `\cite`/math subscripts must be untouched.
//! The section-recovery benefit itself needs the cumulative real-document trigger,
//! so it's verified against the corpus + the fidelity gate, not a synthetic unit.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
    .typst
}

const SENTINEL: char = '\u{1f}';

#[test]
fn label_and_ref_underscore_roundtrip() {
    let t = typ(r"\documentclass{article}\begin{document}\section{S}\label{sec:a_b} See \ref{sec:a_b}.\end{document}");
    assert!(!t.contains(SENTINEL), "sentinel control byte leaked into output:\n{t:?}");
    // The label definition keeps the underscore.
    assert!(t.contains("<sec:a_b>"), "label def lost its underscore:\n{t}");
    // The reference resolves to the same (underscored) key.
    assert!(t.contains("@sec:a_b"), "ref lost its underscore / mismatched:\n{t}");
}

#[test]
fn eqref_underscore_roundtrip() {
    let t = typ(r"\documentclass{article}\begin{document}\begin{equation}x=1\label{eq:x_y}\end{equation} See \eqref{eq:x_y}.\end{document}");
    assert!(!t.contains(SENTINEL), "sentinel leaked:\n{t:?}");
    assert!(t.contains("eq:x_y"), "eqref key lost its underscore:\n{t}");
}

#[test]
fn cite_key_underscore_untouched() {
    // `\cite` keys are matched against the bibliography, so they must NOT be
    // touched by the preprocessing — the underscore stays verbatim.
    let t = typ(r"\documentclass{article}\begin{document}Text \cite{smith_2020}.\end{document}");
    assert!(!t.contains(SENTINEL), "sentinel leaked into a cite:\n{t:?}");
    assert!(t.contains("smith_2020"), "cite key underscore was altered:\n{t}");
}

#[test]
fn math_subscript_underscore_untouched() {
    let t = typ(r"\documentclass{article}\begin{document}$x_1 + y_{ij}$\end{document}");
    assert!(!t.contains(SENTINEL), "sentinel leaked into math:\n{t:?}");
    // Typst subscripts use `_`; they must survive.
    assert!(t.contains("x_1"), "math subscript x_1 broken:\n{t}");
}
