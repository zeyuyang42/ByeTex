//! `\frontmatter`/`\mainmatter` (round-5 dogfood T-frontmatter): in a book/report the
//! frontmatter uses roman page numbers and mainmatter switches to arabic reset to 1.
//! These were dropped (page-numbering lost); now emitted as Typst `#set page(numbering:)`
//! + a page-counter reset. Gated on chapter-bearing classes.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn frontmatter_sets_roman_page_numbers() {
    let t = typ("\\documentclass{book}\\begin{document}\\frontmatter\\chapter{Preface}p\\mainmatter\\chapter{Intro}i\\end{document}");
    assert!(t.contains("#set page(numbering: \"i\")"), "frontmatter → roman; got:\n{t}");
}

#[test]
fn mainmatter_resets_to_arabic() {
    let t = typ("\\documentclass{report}\\begin{document}\\frontmatter\\chapter{P}p\\mainmatter\\chapter{I}i\\end{document}");
    assert!(t.contains("#set page(numbering: \"1\")"), "mainmatter → arabic; got:\n{t}");
    assert!(t.contains("counter(page).update(1)"), "mainmatter resets page counter; got:\n{t}");
}

#[test]
fn non_book_frontmatter_unaffected() {
    // article has no \frontmatter; if present it stays dropped (no page-numbering rules).
    let t = typ("\\documentclass{article}\\begin{document}\\frontmatter body\\end{document}");
    assert!(!t.contains("#set page(numbering: \"i\")"), "article: no roman page rule; got:\n{t}");
}
