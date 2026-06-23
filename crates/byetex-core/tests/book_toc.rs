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
    // Exact form (chapter/section/subsection depth), not just "#outline(" presence.
    assert!(t.contains("#outline(depth: 3)"), "book \\tableofcontents → #outline(depth: 3); got:\n{t}");
    // The outline must come BEFORE the chapter heading it lists — not merely appear somewhere
    // (the old `contains(\"Intro\")` was tautological: Intro is the chapter name from the input).
    let outline_pos = t.find("#outline(").expect("outline present");
    let chapter_pos = t.find("= Intro").expect("chapter heading emitted");
    assert!(outline_pos < chapter_pos, "outline precedes the chapter content; got:\n{t}");
}

#[test]
fn report_tableofcontents_emits_outline() {
    let t = typ("\\documentclass{report}\\begin{document}\\tableofcontents\\chapter{C}body\\end{document}");
    assert!(t.contains("#outline(depth: 3)"), "report \\tableofcontents → #outline(depth: 3); got:\n{t}");
}

#[test]
fn article_tableofcontents_still_dropped() {
    let t = typ("\\documentclass{article}\\begin{document}\\tableofcontents\\section{Intro}x\\end{document}");
    assert!(!t.contains("#outline("), "article \\tableofcontents stays dropped; got:\n{t}");
}
