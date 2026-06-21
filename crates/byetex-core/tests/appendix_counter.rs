//! `\appendix` (round-6 dogfood A7): switched heading numbering to letters but did NOT
//! reset the heading counter, so appendix chapters/sections continued from the prior
//! count (e.g. after 3 chapters the appendices showed D/E instead of A/B). Reset the
//! counter to 0 so the first appendix is A.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn appendix_resets_heading_counter() {
    let t = typ("\\documentclass{book}\\begin{document}\\chapter{One}\\chapter{Two}\\chapter{Three}\\appendix\\chapter{AppA}\\chapter{AppB}\\end{document}");
    assert!(t.contains("#set heading(numbering: \"A.1\")"), "letter numbering set; got:\n{t}");
    assert!(t.contains("#counter(heading).update(0)"), "heading counter reset at \\appendix; got:\n{t}");
    // The reset must come AFTER the numbering set rule and BEFORE the first appendix heading.
    let set_pos = t.find("numbering: \"A.1\"").unwrap();
    let reset_pos = t.find("counter(heading).update(0)").unwrap();
    let appa_pos = t.find("AppA").unwrap();
    assert!(set_pos < reset_pos && reset_pos < appa_pos, "ordering set→reset→appendix; got:\n{t}");
}

#[test]
fn article_appendix_also_resets() {
    // article \appendix turns \section into A, B — same reset needed.
    let t = typ("\\documentclass{article}\\begin{document}\\section{One}\\section{Two}\\appendix\\section{AppA}\\end{document}");
    assert!(t.contains("#counter(heading).update(0)"), "article appendix resets counter; got:\n{t}");
}
