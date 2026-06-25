//! `\begin{verbatim}…\end{verbatim}` was dropped: tree-sitter parses it as its
//! own `verbatim_environment` node (content in a `comment` child), which ByeTex
//! didn't handle, so the body was lost and the `\begin{verbatim}\end{verbatim}`
//! delimiters leaked as text. It should render as a `#raw(block: true)` block,
//! like `lstlisting` (visual grader, gh-klb2-beamer typography slide).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn verbatim_environment_renders_as_raw_block() {
    let src = "\\documentclass{article}\\begin{document}\nBefore.\n\\begin{verbatim}\ndef f(x):\n    return x + 1\n\\end{verbatim}\nAfter.\n\\end{document}";
    let t = typ(src);
    assert!(t.contains("Before.") && t.contains("After."), "lost surrounding text; got:\n{t}");
    assert!(t.contains("#raw("), "verbatim not emitted as #raw; got:\n{t}");
    assert!(t.contains("block: true"), "verbatim should be a raw block; got:\n{t}");
    assert!(t.contains("def f(x):"), "lost verbatim content; got:\n{t}");
    assert!(t.contains("return x + 1"), "lost verbatim content; got:\n{t}");
    // Indentation preserved (verbatim keeps leading spaces).
    assert!(t.contains("    return"), "verbatim should preserve indentation; got:\n{t}");
    assert!(!t.contains(r"\begin{verbatim}"), "leaked the env delimiter; got:\n{t}");
}
