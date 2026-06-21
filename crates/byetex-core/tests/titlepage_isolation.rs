//! `\begin{titlepage}` (round-6 dogfood A6): the env content was emitted as LOOSE body
//! text, so a thesis's inner title page flowed into the following frontmatter/chapter.
//! In LaTeX a titlepage is its own page — isolate it with pagebreaks.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn titlepage_is_pagebreak_isolated() {
    let t = typ("\\documentclass{book}\\begin{document}\\begin{titlepage}\\centering Thesis Title\\end{titlepage}\\chapter{Intro}body\\end{document}");
    assert!(t.contains("Thesis Title"), "titlepage content kept; got:\n{t}");
    // A pagebreak separates the titlepage content from the following chapter.
    let between = t
        .split("Thesis Title")
        .nth(1)
        .and_then(|s| s.split("Intro").next())
        .unwrap_or("");
    assert!(between.contains("pagebreak"), "pagebreak after titlepage, before chapter; got:\n{between}");
}

#[test]
fn titlepage_content_not_glued_to_chapter() {
    let t = typ("\\documentclass{report}\\begin{document}\\begin{titlepage}TITLE\\end{titlepage}\\chapter{C}x\\end{document}");
    // "TITLE= C" (glued) must not happen — there must be a break between.
    assert!(!t.contains("TITLE= C") && !t.contains("TITLE = C"), "title not glued to chapter; got:\n{t}");
}
