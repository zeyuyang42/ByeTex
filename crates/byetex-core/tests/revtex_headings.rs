//! REVTeX/APS section headings are roman-numbered ("I."), uppercase, and
//! centered; subsections use a per-section letter ("A."). ByeTex used arabic
//! "1." left-aligned headings. Found by the visual grader on 2605.31203.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn revtex_headings_roman_centered_uppercase() {
    let t = typ(r"\documentclass[aps]{revtex4-1}\begin{document}\section{Introduction}\subsection{Setup}Body.\end{document}");
    assert!(t.contains("numbering(\"I.\""), "no revtex roman numbering fn; got:\n{t}");
    assert!(t.contains("align(center, upper(it))"), "headings not centered+uppercase; got:\n{t}");
}

#[test]
fn article_headings_stay_arabic_left() {
    let t = typ(r"\documentclass{article}\begin{document}\section{S}x\end{document}");
    assert!(t.contains("numbering: \"1.\"") && !t.contains("numbering(\"I.\""),
        "article should keep plain arabic numbering; got:\n{t}");
}
