//! Beamer `\alt<spec>{default}{alternative}` (B-polish): a static PDF can't switch
//! overlays, so show the DEFAULT (first) arg, drop the spec and the alternative.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn alt_shows_default_drops_alternative_and_spec() {
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\alt<2>{Shown by default.}{Alternative version.}\\end{frame}\\end{document}");
    assert!(t.contains("Shown by default."), "default arg shown; got:\n{t}");
    assert!(!t.contains("Alternative version."), "alternative arg dropped; got:\n{t}");
    assert!(!t.contains("<2>"), "overlay spec must not leak; got:\n{t}");
}

#[test]
fn alt_default_renders_markup() {
    // The default arg goes through the converter (bold etc. survive).
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\alt<1>{\\textbf{Bold here}}{plain}\\end{frame}\\end{document}");
    assert!(t.contains("Bold here"), "default content rendered; got:\n{t}");
    assert!(!t.contains("plain"), "alternative dropped; got:\n{t}");
}

#[test]
fn alt_without_spec_still_keeps_default() {
    // Code-review: no `<spec>` → groups attach as CHILDREN; scanning from after the
    // `\alt` token (not node end) still finds + renders the default, drops the alt.
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\alt{DEFAULTX}{ALTX}\\end{frame}\\end{document}");
    assert!(t.contains("DEFAULTX"), "default kept without a spec; got:\n{t}");
    assert!(!t.contains("ALTX"), "alternative dropped; got:\n{t}");
}

#[test]
fn non_beamer_alt_unaffected() {
    let t = typ("\\documentclass{article}\\begin{document}\\alt<2>{a}{b}\\end{document}");
    // Not the beamer path — must not panic; stable output.
    assert!(t.contains('a') || t.contains('b'), "stable non-beamer handling; got:\n{t}");
}
