//! beamer `columns` with the `\column{width}` COMMAND form (as opposed to
//! nested `column` *environments*). `emit_beamer_columns` only recognised the
//! environment form, so `\begin{columns} \column{w} … \column{w} … \end{columns}`
//! left the `\column` commands unhandled (leaked + cells empty). Common in real
//! decks (corpus gh-klb2-beamer: 5×).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn beamer_column_command_form_splits_into_grid() {
    let src = r"\documentclass{beamer}
\begin{document}
\begin{frame}
\begin{columns}[T,onlytextwidth]
\column{0.5\textwidth}
Left content.
\column{0.5\textwidth}
Right content.
\end{columns}
\end{frame}
\end{document}";
    let t = typ(src);
    assert!(t.contains("#grid"), "no grid emitted; got:\n{t}");
    assert!(t.contains("Left content.") && t.contains("Right content."),
        "lost column content; got:\n{t}");
    assert!(!t.contains(r"\column"), "leaked raw \\column command; got:\n{t}");
    // Two 0.5fr columns.
    assert!(t.contains("0.5fr, 0.5fr"), "column widths not mapped; got:\n{t}");
}
