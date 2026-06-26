//! Nested `itemize`/`enumerate` were FLATTENED: a sub-list's items emitted at
//! column 0 (same level as the parent) and glued to the parent item's text
//! (`+ A+ A1`), so Typst lost the hierarchy. Indent nested list markers by two
//! spaces per level and start the sub-list on its own line. Found by direct
//! validation (~19 corpus papers use nested lists).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn nested_enumerate_is_indented_and_not_glued() {
    let t = typ(r"\begin{enumerate}\item A\begin{enumerate}\item A1\item A2\end{enumerate}\item B\end{enumerate}");
    assert!(t.contains("  + A1"), "nested item not indented; got:\n{t}");
    assert!(t.contains("  + A2"), "second nested item not indented; got:\n{t}");
    assert!(!t.contains("A+ A1"), "nested list glued to parent item; got:\n{t}");
}

#[test]
fn nested_itemize_is_indented() {
    let t = typ(r"\begin{itemize}\item A\begin{itemize}\item inner\end{itemize}\end{itemize}");
    assert!(t.contains("  - inner"), "nested itemize not indented; got:\n{t}");
}

#[test]
fn top_level_list_not_indented() {
    let t = typ(r"\begin{itemize}\item one\item two\end{itemize}");
    assert!(t.contains("- one") && !t.contains("  - one"), "top-level over-indented; got:\n{t}");
}
