//! Beamer `block`/`alertblock`/`exampleblock` (B2): titled callout boxes that used
//! to be dropped (`unsupported_environment`). Now each maps to a titled `#block`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = r#"\documentclass{beamer}
\begin{document}
\begin{frame}{Blocks}
\begin{block}{Definition}
A key definition here.
\end{block}
\begin{alertblock}{Warning}
Be careful here.
\end{alertblock}
\begin{exampleblock}{Example}
For instance, this.
\end{exampleblock}
\end{frame}
\end{document}"#;

#[test]
fn block_content_and_titles_kept() {
    let t = typ(DECK);
    for s in [
        "Definition", "A key definition here.",
        "Warning", "Be careful here.",
        "Example", "For instance, this.",
    ] {
        assert!(t.contains(s), "block content `{s}` kept; got:\n{t}");
    }
}

#[test]
fn blocks_become_typst_blocks() {
    let t = typ(DECK);
    // Three callout boxes → three #block(...) calls.
    assert_eq!(t.matches("#block(").count(), 3, "one #block per beamer block; got:\n{t}");
}

#[test]
fn non_beamer_block_env_unaffected() {
    // Gated on the beamer class — a non-beamer `block` env keeps its old handling.
    let t = typ("\\documentclass{article}\\begin{document}\\begin{block}{T}x\\end{block}\\end{document}");
    assert!(!t.contains("#block("), "non-beamer block must not become a #block; got:\n{t}");
}
