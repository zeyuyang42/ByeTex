//! `\texttt{\textit{X}}` / `\texttt{\textbf{X}}` rendered the nested font-switch
//! as a literal string — `#raw("\textit{X}")` — because `\texttt` emits its
//! argument verbatim. Peel `\textit`/`\textbf`/`\emph` wrappers and emit a
//! styled monospace form: `#text(style/weight)[#raw("X")]`. Plain `\texttt{X}`
//! is unchanged. Found by the visual grader on gh-klb2-beamer (font-feature slide).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn plain_texttt_unchanged() {
    let t = typ(r"\documentclass{article}\begin{document}\texttt{plain}\end{document}");
    assert!(t.contains(r#"#raw("plain")"#), "plain texttt changed; got:\n{t}");
}

#[test]
fn texttt_italic_renders_styled_not_literal() {
    let t = typ(r"\documentclass{article}\begin{document}\texttt{\textit{Mono Italic}}\end{document}");
    assert!(!t.contains(r"\textit"), "leaked \\textit literal; got:\n{t}");
    assert!(t.contains(r#"style: "italic""#), "italic style not applied; got:\n{t}");
    assert!(t.contains(r#"#raw("Mono Italic")"#), "lost monospace content; got:\n{t}");
}

#[test]
fn texttt_bold_renders_styled_not_literal() {
    let t = typ(r"\documentclass{article}\begin{document}\texttt{\textbf{Mono Bold}}\end{document}");
    assert!(!t.contains(r"\textbf"), "leaked \\textbf literal; got:\n{t}");
    assert!(t.contains(r#"weight: "bold""#), "bold weight not applied; got:\n{t}");
    assert!(t.contains(r#"#raw("Mono Bold")"#), "lost monospace content; got:\n{t}");
}

#[test]
fn texttt_bold_italic_combines() {
    let t = typ(r"\documentclass{article}\begin{document}\texttt{\textbf{\textit{BI}}}\end{document}");
    assert!(!t.contains(r"\text"), "leaked a font command literal; got:\n{t}");
    assert!(t.contains(r#"weight: "bold""#) && t.contains(r#"style: "italic""#), "bold+italic not combined; got:\n{t}");
    assert!(t.contains(r#"#raw("BI")"#), "lost content; got:\n{t}");
}
