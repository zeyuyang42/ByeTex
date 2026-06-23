//! Beamer aspect ratio (B7 fidelity): beamer's DEFAULT slide is 4:3, and decks opt
//! into widescreen with `[aspectratio=169]`. With the touying emitter (Phase 3a) the
//! ratio surfaces as the `aspect-ratio:` argument of `metropolis-theme.with(…)`, not a
//! `#set page(paper: "presentation-…")` value.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn default_beamer_is_4_3() {
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{S}x\\end{frame}\\end{document}");
    assert!(t.contains("aspect-ratio: \"4-3\""), "default beamer is 4:3; got:\n{t}");
    assert!(!t.contains("aspect-ratio: \"16-9\""), "default is not widescreen");
}

#[test]
fn aspectratio_169_is_widescreen() {
    let t = typ("\\documentclass[aspectratio=169]{beamer}\\begin{document}\\begin{frame}{S}x\\end{frame}\\end{document}");
    assert!(t.contains("aspect-ratio: \"16-9\""), "aspectratio=169 → 16:9; got:\n{t}");
}

#[test]
fn aspectratio_43_is_standard() {
    let t = typ("\\documentclass[aspectratio=43]{beamer}\\begin{document}\\begin{frame}{S}x\\end{frame}\\end{document}");
    assert!(t.contains("aspect-ratio: \"4-3\""), "aspectratio=43 → 4:3; got:\n{t}");
}
