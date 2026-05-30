//! Regression tests for sections with multiple consecutive \label{} commands.
//! LaTeX allows multiple \label{} on the same section (all refer to it).
//! Typst only supports one label per element (last wins, rest are silently
//! dropped). ByeTex must consume extra sibling labels so they don't float
//! free and win via Typst's "last label wins" rule. Paper 22800 regression.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn first_label_kept_on_section() {
    let src = r"\documentclass{article}
\begin{document}
\subsection*{Headline Block}
\label{sec:headline}
\label{sec:T7A-headline}

Body.
\end{document}";
    let out = convert(src);
    // First label must appear on the heading line.
    assert!(
        out.typst.contains("<sec:headline>"),
        "first label must be present, got: {}",
        out.typst
    );
    // Second (extra) label must NOT appear as a standalone element.
    // If it does, Typst's "last label wins" drops <sec:headline>.
    assert!(
        !out.typst.contains("<sec:T7A-headline>"),
        "extra sibling label must be consumed (not emitted), got: {}",
        out.typst
    );
}

#[test]
fn three_sibling_labels_only_first_survives() {
    let src = r"\documentclass{article}
\begin{document}
\section{Introduction}
\label{sec:intro}
\label{sec:introduction}
\label{sec:intro-alias}

Text.
\end{document}";
    let out = convert(src);
    assert!(
        out.typst.contains("<sec:intro>"),
        "first label must survive, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("<sec:introduction>"),
        "second extra label must be consumed, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("<sec:intro-alias>"),
        "third extra label must be consumed, got: {}",
        out.typst
    );
}

#[test]
fn starred_section_with_label_uses_function_numbering() {
    // A \subsection* with a \label must use numbering: (..n) => none so that
    // Typst's @ref works. numbering: none makes the heading unreferenceable in
    // Typst 0.14+. The function keeps the heading in the counter (references
    // work) but renders no visible number. Paper 22800 regression.
    let src = r"\documentclass{article}
\begin{document}
\subsection*{Unnumbered but referenced}
\label{sec:unnumbered}

See @sec:unnumbered above.
\end{document}";
    let out = convert(src);
    // Must use the function-based numbering (not none, not "")
    assert!(
        out.typst.contains("numbering: (..n) => none"),
        "starred section with label must use 'numbering: (..n) => none', got: {}",
        out.typst
    );
    // Must contain the label
    assert!(
        out.typst.contains("<sec:unnumbered>"),
        "label must be present, got: {}",
        out.typst
    );
}
