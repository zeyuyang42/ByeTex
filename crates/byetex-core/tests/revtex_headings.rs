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

#[test]
fn revtex_deep_headings_not_misnumbered() {
    // The numbering closure must not blindly reuse p.at(2) for depth >= 4
    // (that mis-numbered \paragraph). RevTeX's secnumdepth is 3, so deeper
    // levels are unnumbered (none).
    let t = typ(r"\documentclass[aps]{revtex4-1}\begin{document}\section{S}\subsection{Sub}\subsubsection{SubSub}\paragraph{Para}x\end{document}");
    assert!(
        t.contains("p.len() == 3"),
        "deep heading levels should be handled explicitly, not via a catch-all p.at(2); got:\n{t}"
    );
}
