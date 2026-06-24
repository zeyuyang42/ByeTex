//! `\DeclarePairedDelimiter{\name}{L}{R}` (mathtools) defines `\name{x}` → `L x R`.
//! ByeTex didn't handle it, so the declaration's delimiter arguments (`\vert`,
//! `\lceil`, …) leaked as `unsupported_command` and `\abs{x}` never expanded
//! (corpus 2605.30609 / 2605.31203 / gh-klb2-beamer).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn paired_delimiter_braced_name_expands() {
    let src = r"\documentclass{article}
\DeclarePairedDelimiter{\abs}{\vert}{\vert}
\begin{document}
$\abs{x}$
\end{document}";
    let out = convert(src, &ConvertOptions::default());
    // `\abs{x}` → `| x |` (the `\vert`s map to `|`), no leaked `\abs`/`\vert`.
    assert!(out.typst.contains("|x|") || out.typst.contains("| x |") || out.typst.contains("|"),
        "abs did not expand to vertical bars; got:\n{}", out.typst);
    assert!(!out.typst.contains(r"\abs") && !out.typst.contains(r"\vert"),
        "leaked raw macro/delimiter; got:\n{}", out.typst);
    assert!(out.warnings.is_empty(), "unexpected warnings: {:?}", out.warnings);
}

#[test]
fn paired_delimiter_unbraced_name_and_ceil() {
    let src = r"\documentclass{article}
\DeclarePairedDelimiter\ceil{\lceil}{\rceil}
\begin{document}
$\ceil{y}$
\end{document}";
    let t = typ(src);
    assert!(t.contains("ceil.l") && t.contains("ceil.r"),
        "ceil delimiters missing; got:\n{t}");
    assert!(t.contains("y"), "argument lost; got:\n{t}");
    assert!(!t.contains(r"\ceil") && !t.contains(r"\lceil"),
        "leaked raw macro; got:\n{t}");
}
