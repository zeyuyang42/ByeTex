//! `\label{key_with_underscore}` inside a theorem-like env leaked its tail as body
//! text: tree-sitter truncates the label key at the first `_`, so `\label{prop:loo`
//! parsed as the label and `_to_denoiser}` leaked into the proposition body
//! (round-4 dogfood A2). The whole `\label{…}` must be consumed.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const SRC: &str = "\\documentclass{article}\\newtheorem{proposition}{Proposition}\\begin{document}\\begin{proposition}\\label{prop:loo_to_denoiser}\nThe statement holds.\\end{proposition}\\end{document}";

#[test]
fn underscore_label_tail_does_not_leak() {
    let t = typ(SRC);
    assert!(t.contains("The statement holds."), "body kept; got:\n{t}");
    // The truncated tail must NOT leak into the figure body.
    assert!(!t.contains("\\_to") && !t.contains("_to_denoiser\n") && !t.contains("[\\_"),
        "label tail must not leak into the body; got:\n{t}");
}

#[test]
fn underscore_label_still_attaches() {
    let t = typ(SRC);
    // The full label key is still attached for cross-references.
    assert!(t.contains("<prop:loo_to_denoiser>"), "full label attached; got:\n{t}");
}

#[test]
fn plain_label_unaffected() {
    // Control: a label with no underscore still works.
    let t = typ("\\documentclass{article}\\newtheorem{lem}{Lemma}\\begin{document}\\begin{lem}\\label{lemA}\nBody.\\end{lem}\\end{document}");
    assert!(t.contains("<lemA>"), "plain label attached; got:\n{t}");
    assert!(t.contains("Body."), "body kept");
}
