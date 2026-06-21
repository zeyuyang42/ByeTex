//! Book/report `\tableofcontents` (round-5 dogfood T-toc): renders a `#outline` of the
//! chapters/sections instead of being dropped (B-toc was beamer-only). Article-family
//! keeps the prior drop (a paper ToC is rare; avoid surprising two-column layouts).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn book_tableofcontents_emits_outline() {
    let t = typ("\\documentclass{book}\\begin{document}\\tableofcontents\\chapter{Intro}\\section{S}body\\end{document}");
    assert!(t.contains("#outline("), "book \\tableofcontents → #outline; got:\n{t}");
    assert!(t.contains("Intro"), "chapters present");
}

#[test]
fn report_tableofcontents_emits_outline() {
    let t = typ("\\documentclass{report}\\begin{document}\\tableofcontents\\chapter{C}body\\end{document}");
    assert!(t.contains("#outline("), "report \\tableofcontents → #outline; got:\n{t}");
}

#[test]
fn article_tableofcontents_still_dropped() {
    let t = typ("\\documentclass{article}\\begin{document}\\tableofcontents\\section{Intro}x\\end{document}");
    assert!(!t.contains("#outline("), "article \\tableofcontents stays dropped; got:\n{t}");
}
