//! Thesis/book density: in a chapter-bearing class (`book`/`report`/thesis), every
//! `\chapter` (and `\chapter*`) issues a `\clearpage` in LaTeX, so each chapter STARTS
//! ON A NEW PAGE. ByeTex now emits a `#pagebreak(weak: true)` before each top-level
//! (level-1) chapter heading so converted theses get the same page density as the truth
//! (was ~half the page count because chapters packed together). A `weak` break collapses
//! against an existing break (cover/frontmatter), so no blank first page.
//!
//! Articles (no chapters) must NOT pagebreak: sections stay inline. Only level-1
//! (`\chapter`/`\part`) breaks; `\section`/`\subsection` (level >= 2) do not.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn book_chapter_pagebreaks_before_each_chapter() {
    let t = typ("\\documentclass{book}\\begin{document}\\chapter{One}a\\chapter{Two}b\\end{document}");
    // One pagebreak per chapter (two chapters → two weak breaks).
    let n = t.matches("#pagebreak(weak: true)").count();
    assert_eq!(n, 2, "each \\chapter gets a weak pagebreak; got {n}:\n{t}");
    // The break precedes the heading, not after it.
    let pb = t.find("#pagebreak(weak: true)").unwrap();
    let one = t.find("= One").unwrap();
    assert!(pb < one, "pagebreak comes before the chapter heading; got:\n{t}");
}

#[test]
fn report_starred_chapter_also_pagebreaks() {
    // Frontmatter Preface/Summary/Nomenclature are `\chapter*` — they must break too.
    let t = typ("\\documentclass{report}\\begin{document}\\chapter*{Preface}p\\chapter{Intro}i\\end{document}");
    let n = t.matches("#pagebreak(weak: true)").count();
    assert_eq!(n, 2, "starred + numbered chapters both break; got {n}:\n{t}");
    // The starred chapter (a #heading call) is preceded by a break.
    let pb = t.find("#pagebreak(weak: true)").unwrap();
    let pref = t.find("Preface").unwrap();
    assert!(pb < pref, "pagebreak precedes the starred chapter; got:\n{t}");
}

#[test]
fn book_section_does_not_pagebreak() {
    // \section (level 2 in a book) must NOT pagebreak — only chapters do.
    let t = typ("\\documentclass{book}\\begin{document}\\chapter{Ch}a\\section{Sec}b\\section{Sec2}c\\end{document}");
    // One chapter → exactly one break, regardless of the two sections.
    let n = t.matches("#pagebreak(weak: true)").count();
    assert_eq!(n, 1, "only the chapter breaks, not the two sections; got {n}:\n{t}");
}

#[test]
fn article_section_does_not_pagebreak() {
    // Article (no chapters) — sections stay inline, no breaks.
    let t = typ("\\documentclass{article}\\begin{document}\\section{One}a\\section{Two}b\\end{document}");
    assert!(
        !t.contains("#pagebreak(weak: true)"),
        "article sections never pagebreak; got:\n{t}"
    );
}

#[test]
fn book_part_pagebreaks() {
    // \part is also level 1 in a chapter-bearing class → breaks.
    let t = typ("\\documentclass{book}\\begin{document}\\part{P}\\chapter{C}a\\end{document}");
    let n = t.matches("#pagebreak(weak: true)").count();
    assert_eq!(n, 2, "\\part and \\chapter both break; got {n}:\n{t}");
}
