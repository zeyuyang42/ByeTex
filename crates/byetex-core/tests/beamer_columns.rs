//! Beamer `columns`/`column` (B1): the two-column slide layout used to be dropped
//! (`unsupported_environment`), losing the column content. Now it maps to a Typst
//! `#grid` so both columns and their widths survive.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = r#"\documentclass{beamer}
\begin{document}
\begin{frame}{Two Columns}
\begin{columns}
\begin{column}{0.5\textwidth}
Left content here.
\end{column}
\begin{column}{0.4\textwidth}
Right content here.
\end{column}
\end{columns}
\end{frame}
\end{document}"#;

#[test]
fn column_content_is_kept() {
    let t = typ(DECK);
    assert!(t.contains("Left content here."), "left column kept; got:\n{t}");
    assert!(t.contains("Right content here."), "right column kept");
}

#[test]
fn columns_become_a_grid() {
    let t = typ(DECK);
    assert!(t.contains("#grid("), "columns → #grid; got:\n{t}");
    // `\textwidth`-relative widths become fr ratios.
    assert!(t.contains("0.5fr") && t.contains("0.4fr"), "column widths mapped to fr; got:\n{t}");
}

#[test]
fn leading_dot_width_is_valid_typst() {
    // Code-review: `{.45\textwidth}` (no leading zero, idiomatic beamer) must become
    // `0.45fr`, not `.45fr` (which Typst rejects → whole doc fails to compile).
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{X}\\begin{columns}\\begin{column}{.45\\textwidth}A\\end{column}\\begin{column}{.45\\textwidth}B\\end{column}\\end{columns}\\end{frame}\\end{document}");
    assert!(t.contains("0.45fr"), "leading-dot width normalized; got:\n{t}");
    assert!(!t.contains("(.45fr") && !t.contains(" .45fr"), "no bare `.45fr`; got:\n{t}");
}

#[test]
fn non_beamer_columns_unaffected() {
    // The handler is gated on the beamer class.
    let t = typ("\\documentclass{article}\\begin{document}\\begin{columns}\\begin{column}{0.5\\textwidth}x\\end{column}\\end{columns}\\end{document}");
    assert!(!t.contains("#grid("), "non-beamer columns must not become a grid; got:\n{t}");
}
