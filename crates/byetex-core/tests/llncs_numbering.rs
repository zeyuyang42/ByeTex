//! Springer LNCS section headings have NO trailing period ("1 Introduction",
//! "1.1 Setup"); ByeTex used "1." (trailing period). Typst numbering "1.1"
//! yields "1"/"1.1"/"1.1.1" with no trailing period. Found by the visual grader
//! on 2605.31597.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn llncs_heading_numbering_has_no_trailing_period() {
    let t = typ(r"\documentclass{llncs}\begin{document}\section{Intro}\subsection{Setup}x\end{document}");
    assert!(t.contains("#set heading(numbering: \"1.1\")"), "llncs should use no-period numbering; got:\n{t}");
}

#[test]
fn article_keeps_trailing_period_numbering() {
    let t = typ(r"\documentclass{article}\begin{document}\section{S}x\end{document}");
    assert!(t.contains("#set heading(numbering: \"1.\")"), "article keeps 1.; got:\n{t}");
}
