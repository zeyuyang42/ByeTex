//! `\chapter` starts on a RECTO page in a twoside book.
//!
//! LaTeX's `book` defaults to `twoside,openright`, so `\chapter` issues
//! `\cleardoublepage` and leaves a blank verso. gh-amberj-latex-book-template's
//! truth has 6 near-empty pages of 16; ours had 1 of 9, which is most of why that
//! paper was the corpus's worst structure FAIL (page_ratio 0.56).
//!
//! DELIBERATELY NARROW. Applied blanketly to chapter-bearing documents this is
//! destructive: gh-dzwaneveld-tudelft-thesis goes 11 -> 21 pages against a truth
//! of 12, because its class is effectively one-sided and it has five level-1
//! headings. The discriminator is that a bare `\documentclass{book}` is
//! twoside+openright, while any source declaring `oneside`/`openany` is not.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const BODY: &str = "\\begin{document}\n\\chapter{One}\nText.\n\\chapter{Two}\nMore.\n\\end{document}";

#[test]
fn a_plain_book_starts_chapters_recto() {
    let t = typ(&format!("\\documentclass{{book}}\n{BODY}"));
    assert!(
        t.contains("#pagebreak(to: \"odd\""),
        "book is twoside+openright by default; got:\n{t}"
    );
}

#[test]
fn oneside_disables_it() {
    // The tudelft thesis case: its class declares `oneside`, its truth has one
    // near-empty page of 12, and forcing recto starts nearly doubled our output.
    let t = typ(&format!("\\documentclass[oneside]{{book}}\n{BODY}"));
    assert!(
        !t.contains("to: \"odd\""),
        "an explicit oneside must disable recto starts; got:\n{t}"
    );
}

#[test]
fn openany_disables_it_too() {
    let t = typ(&format!("\\documentclass[openany]{{book}}\n{BODY}"));
    assert!(!t.contains("to: \"odd\""), "openany means chapters open anywhere; got:\n{t}");
}

#[test]
fn an_article_is_untouched() {
    // The control: no chapters, no recto starts, and none of the 59 arXiv papers
    // is affected.
    let t = typ("\\documentclass{article}\n\\begin{document}\n\\section{One}\nText.\n\\end{document}");
    assert!(!t.contains("to: \"odd\""), "articles are unaffected; got:\n{t}");
}
