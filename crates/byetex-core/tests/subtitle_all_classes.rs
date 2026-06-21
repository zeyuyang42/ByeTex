//! `\subtitle{…}` was captured only for beamer (#329) and dropped for every other
//! class, losing the subtitle on report/book/thesis title pages (round-5 dogfood T1).
//! It's a title-block element; capture + render it under the title for any class.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn report_subtitle_is_rendered() {
    let t = typ("\\documentclass{report}\\title{T}\\subtitle{My Subtitle}\\author{A}\\begin{document}\\maketitle\\end{document}");
    assert!(t.contains("My Subtitle"), "report subtitle rendered under the title; got:\n{t}");
}

#[test]
fn book_subtitle_is_rendered() {
    let t = typ("\\documentclass{book}\\title{T}\\subtitle{Vol Two}\\author{A}\\begin{document}\\maketitle\\end{document}");
    assert!(t.contains("Vol Two"), "book subtitle rendered; got:\n{t}");
}

#[test]
fn article_subtitle_is_rendered() {
    // An article that does use \subtitle should keep it too (no harm).
    let t = typ("\\documentclass{article}\\title{T}\\subtitle{The Sub}\\author{A}\\begin{document}\\maketitle\\end{document}");
    assert!(t.contains("The Sub"), "article subtitle rendered; got:\n{t}");
}
