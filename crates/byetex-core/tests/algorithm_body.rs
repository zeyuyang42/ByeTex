//! An `algorithm` float wrapping `\begin{algorithmic}` pseudocode used to render
//! as an empty `(figure)` placeholder — the entire body was dropped, leaving the
//! agent nothing to translate (dogfood backlog F7; corpus 2605.31510 had 3 algos,
//! 2605.22728 had 5). The algorithmic steps are now rendered as the float body.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const ALG: &str = r"\documentclass{article}\usepackage{algorithm}\usepackage{algorithmic}
\begin{document}
\begin{algorithm}
\caption{My Method}
\label{alg:m}
\begin{algorithmic}
\STATE Initialize the accumulator
\STATE Return the result
\end{algorithmic}
\end{algorithm}
\end{document}";

#[test]
fn algorithm_body_is_preserved_not_placeholder() {
    let t = typ(ALG);
    assert!(
        t.contains("Initialize the accumulator") && t.contains("Return the result"),
        "the pseudocode steps must survive; got:\n{t}"
    );
    assert!(
        !t.contains("[(figure)]"),
        "must not fall back to the empty figure placeholder; got:\n{t}"
    );
}

#[test]
fn algorithm_keeps_caption_and_label() {
    let t = typ(ALG);
    assert!(t.contains("caption: [My Method]"), "caption preserved; got:\n{t}");
    assert!(t.contains("<alg:m>"), "label preserved; got:\n{t}");
}

#[test]
fn multiple_algorithmic_blocks_all_render() {
    // A float with two `algorithmic` blocks must keep BOTH (code-review finding:
    // the first draft captured only the first).
    let t = typ(
        "\\documentclass{article}\\usepackage{algorithm}\\usepackage{algorithmic}\n\
         \\begin{document}\\begin{algorithm}\\caption{Two}\n\
         \\begin{algorithmic}\\STATE FIRST block step\\end{algorithmic}\n\
         \\begin{algorithmic}\\STATE SECOND block step\\end{algorithmic}\n\
         \\end{algorithm}\\end{document}",
    );
    assert!(t.contains("FIRST block step"), "first block must render; got:\n{t}");
    assert!(t.contains("SECOND block step"), "second block must render too; got:\n{t}");
}

#[test]
fn plain_figure_still_placeholders_when_empty() {
    // Regression: a genuinely empty figure (no graphic/tabular/algorithmic) must
    // still get the needs_manual_review placeholder, not crash.
    let t = typ(r"\documentclass{article}\begin{document}\begin{figure}\caption{X}\end{figure}\end{document}");
    assert!(t.contains("caption: [X]"), "empty figure still emits caption; got:\n{t}");
}
