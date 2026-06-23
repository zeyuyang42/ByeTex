//! `\tableofcontents` outline depth follows `\setcounter{tocdepth}{N}` (health-check P4).
//! LaTeX book/report tocdepth (0=chapter,1=section,2=subsection,3=subsubsection) maps to the
//! Typst outline depth = tocdepth+1 (chapter=1,section=2,…). No tocdepth → the default depth 3.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn tocdepth_1_limits_outline_to_depth_2() {
    let t = typ("\\documentclass{book}\\setcounter{tocdepth}{1}\\begin{document}\\tableofcontents\\chapter{C}\\section{S}x\\end{document}");
    assert!(t.contains("#outline(depth: 2)"), "tocdepth 1 → #outline(depth: 2); got:\n{t}");
}

#[test]
fn tocdepth_2_gives_depth_3() {
    let t = typ("\\documentclass{report}\\setcounter{tocdepth}{2}\\begin{document}\\tableofcontents\\chapter{C}x\\end{document}");
    assert!(t.contains("#outline(depth: 3)"), "tocdepth 2 → #outline(depth: 3); got:\n{t}");
}

#[test]
fn no_tocdepth_keeps_default_depth_3() {
    // Byte-identical to today: the common case must not change.
    let t = typ("\\documentclass{book}\\begin{document}\\tableofcontents\\chapter{C}x\\end{document}");
    assert!(t.contains("#outline(depth: 3)"), "no tocdepth → default depth 3; got:\n{t}");
}
