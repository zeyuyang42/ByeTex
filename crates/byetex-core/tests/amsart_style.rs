//! amsart (AMS math-journal class) uppercases the title (\MakeUppercase) and
//! CENTERS section headings. ByeTex rendered the title in as-typed case and
//! left-aligned headings. Found by the visual grader on 2605.22485.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DOC: &str = r"\documentclass{amsart}\title{My Paper Title}\author{A}\begin{document}\maketitle\section{Intro}Body.\end{document}";

#[test]
fn amsart_title_is_uppercased() {
    let t = typ(DOC);
    assert!(t.contains("upper["), "amsart title not uppercased; got:\n{t}");
}

#[test]
fn amsart_section_headings_centered() {
    let t = typ(DOC);
    assert!(t.contains("heading.where(level: 1): it => align(center"), "amsart headings not centered; got:\n{t}");
}

#[test]
fn article_title_not_uppercased() {
    let t = typ(r"\documentclass{article}\title{My Title}\author{A}\begin{document}\maketitle\section{S}Body.\end{document}");
    assert!(!t.contains("upper["), "non-amsart title should not be uppercased; got:\n{t}");
}
