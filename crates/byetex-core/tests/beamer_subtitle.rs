//! Beamer `\subtitle{…}` (round-4 B-subtitle): rendered under the title on the title
//! slide instead of being dropped. Gated on beamer (papers have no subtitle slot).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn beamer_subtitle_is_rendered() {
    let t = typ("\\documentclass{beamer}\\title{My Talk}\\subtitle{An Empirical Study}\\author{A}\\begin{document}\\frame{\\titlepage}\\end{document}");
    assert!(t.contains("My Talk"), "title rendered");
    assert!(t.contains("An Empirical Study"), "subtitle rendered under the title; got:\n{t}");
}

#[test]
fn beamer_subtitle_alone_makes_title_block() {
    // A deck with only \title + \subtitle (no author) still renders the subtitle.
    let t = typ("\\documentclass{beamer}\\title{T}\\subtitle{SUBONLY}\\begin{document}\\frame{\\titlepage}\\end{document}");
    assert!(t.contains("SUBONLY"), "subtitle rendered; got:\n{t}");
}

#[test]
fn non_beamer_subtitle_is_now_rendered() {
    // Updated (round-5 T1): `\subtitle` is a title-block element for any class now, not
    // beamer-only — a report/book/article subtitle is rendered, not dropped.
    let t = typ("\\documentclass{article}\\title{T}\\subtitle{KEEPME}\\author{A}\\begin{document}\\maketitle\\end{document}");
    assert!(t.contains("KEEPME"), "non-beamer subtitle now rendered; got:\n{t}");
}
