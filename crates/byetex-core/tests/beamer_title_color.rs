//! Beamer frame-title color is DETECTED from the deck's theme, not hard-coded:
//! `\setbeamercolor{frametitle|structure}{fg=…}` and `\definecolor` are honored
//! exactly; `\usecolortheme{name}` maps to the theme's structure color; a stock deck
//! (no theme) falls back to beamer's default structure blue.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn deck(preamble: &str) -> String {
    typ(&format!(
        "\\documentclass{{beamer}}{preamble}\\begin{{document}}\\begin{{frame}}{{Method}}x\\end{{frame}}\\end{{document}}"
    ))
}

// beamer default structure color rgb(0.2,0.2,0.7) ≈ #3333b3.
const DEFAULT_BLUE: &str = "#3333b3";

#[test]
fn default_theme_is_structure_blue() {
    let t = deck("");
    assert!(t.contains(DEFAULT_BLUE), "stock deck → default blue; got:\n{t}");
}

#[test]
fn explicit_frametitle_color_is_honored() {
    let t = deck("\\setbeamercolor{frametitle}{fg=green}");
    assert!(t.contains("Method"), "title rendered");
    assert!(!t.contains(DEFAULT_BLUE), "explicit color overrides the default blue; got:\n{t}");
}

#[test]
fn definecolor_frametitle_is_resolved() {
    let t = deck("\\definecolor{brand}{RGB}{200,0,0}\\setbeamercolor{frametitle}{fg=brand}");
    // The custom red is used, not the default blue.
    assert!(!t.contains(DEFAULT_BLUE), "\\definecolor brand used; got:\n{t}");
    assert!(t.contains("200") || t.contains("c80000") || t.contains("C80000"),
        "brand RGB(200,0,0) resolved; got:\n{t}");
}

#[test]
fn colortheme_beaver_is_not_blue() {
    let t = deck("\\usecolortheme{beaver}");
    assert!(!t.contains(DEFAULT_BLUE), "beaver is red, not the default blue; got:\n{t}");
}

#[test]
fn non_beamer_unaffected() {
    let t = typ("\\documentclass{article}\\begin{document}\\section{Intro}x\\end{document}");
    assert!(!t.contains(DEFAULT_BLUE), "non-beamer headings are not slide-blue; got:\n{t}");
}
