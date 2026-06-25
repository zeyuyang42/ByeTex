//! `\metroset{...}` (the beamer `metropolis` theme's config command, e.g.
//! `\metroset{block=fill}`) is presentation-only styling with no document
//! output. It was flagged `unsupported_command` in every metropolis deck
//! (gh-klb2-beamer / gh-mtheme-demo / gh-bard-metropolis); drop it silently
//! with its argument.

use byetex_core::{convert, ConvertOptions};

#[test]
fn metroset_dropped_silently_with_arg() {
    let src = r"\documentclass{beamer}
\metroset{block=fill,sectionpage=progressbar}
\begin{document}
\begin{frame}
Body text.
\end{frame}
\end{document}";
    let out = convert(src, &ConvertOptions::default());
    assert!(out.typst.contains("Body text."), "lost body; got:\n{}", out.typst);
    assert!(!out.typst.contains("metroset"), "leaked \\metroset; got:\n{}", out.typst);
    assert!(!out.typst.contains("block=fill"), "leaked \\metroset arg; got:\n{}", out.typst);
    assert!(out.warnings.is_empty(), "unexpected warnings: {:?}", out.warnings);
}
