//! Beamer frame-title color. Phase 3a (touying): the metropolis theme owns the dark
//! header-bar color of every frame, so the converter no longer paints frame titles with
//! a detected `#text(fill: …)` color — the title is a plain `== heading` and touying
//! styles it. The theme-color → touying `config-colors` mapping (honoring
//! `\setbeamercolor` / `\usecolortheme`) is Phase 3b. These tests pin the 3a contract:
//! detection commands are still consumed (no `unsupported` warning, no leak), and the
//! frame title is a heading with no hand-rolled color.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn deck(preamble: &str) -> String {
    typ(&format!(
        "\\documentclass{{beamer}}{preamble}\\begin{{document}}\\begin{{frame}}{{Method}}x\\end{{frame}}\\end{{document}}"
    ))
}

#[test]
fn frame_title_is_a_heading_not_a_colored_text() {
    let t = deck("");
    assert!(t.contains("== Method"), "frame title is a touying `==` slide; got:\n{t}");
    // 3a: no hand-rolled colored frame-title text (touying owns the header color).
    assert!(
        !t.contains("#text(size: 1.2em, weight: \"bold\", fill:"),
        "no converter-painted frame-title color in 3a; got:\n{t}"
    );
}

#[test]
fn setbeamercolor_is_consumed_no_leak() {
    // `\setbeamercolor{frametitle}{fg=green}` is still parsed/consumed (it must not leak
    // as body text), even though its color is not applied in 3a.
    let t = deck("\\setbeamercolor{frametitle}{fg=green}");
    assert!(t.contains("== Method"), "title rendered; got:\n{t}");
    assert!(!t.contains("setbeamercolor"), "command must not leak; got:\n{t}");
    assert!(!t.contains("fg=green"), "raw spec must not leak; got:\n{t}");
}

#[test]
fn definecolor_and_usecolortheme_do_not_leak() {
    let t = deck("\\definecolor{brand}{RGB}{200,0,0}\\setbeamercolor{frametitle}{fg=brand}\\usecolortheme{beaver}");
    assert!(t.contains("== Method"), "title rendered; got:\n{t}");
    assert!(!t.contains("definecolor"), "\\definecolor must not leak; got:\n{t}");
    assert!(!t.contains("usecolortheme"), "\\usecolortheme must not leak; got:\n{t}");
}

#[test]
fn non_beamer_unaffected() {
    let t = typ("\\documentclass{article}\\begin{document}\\section{Intro}x\\end{document}");
    assert!(!t.contains("metropolis-theme"), "non-beamer is not a slide deck; got:\n{t}");
}

// ─── Phase 3b: theme color → touying `config-colors` ────────────────────────────────

#[test]
fn no_color_detected_leaves_metropolis_default() {
    // A bare beamer deck (no colortheme / setbeamercolor) must NOT emit a color override —
    // detect-don't-hardcode: leave the metropolis default untouched.
    let t = deck("");
    assert!(t.contains("metropolis-theme.with("), "metropolis theme applied; got:\n{t}");
    assert!(
        !t.contains("config-colors("),
        "no detected color → no config-colors override (metropolis default kept); got:\n{t}"
    );
}

#[test]
fn usecolortheme_beaver_maps_to_primary() {
    // `\usecolortheme{beaver}` is beamer's red theme; its structure color maps onto the
    // metropolis accent (`primary`).
    let t = deck("\\usecolortheme{beaver}");
    assert!(
        t.contains("config-colors(primary: rgb(\"#a62640\"))"),
        "beaver → red primary override; got:\n{t}"
    );
}

#[test]
fn setbeamercolor_structure_maps_to_primary() {
    let t = deck("\\setbeamercolor{structure}{fg=green}");
    assert!(
        t.contains("config-colors(primary: green)"),
        "structure fg=green → primary override; got:\n{t}"
    );
    assert!(!t.contains("fg=green"), "raw spec must not leak; got:\n{t}");
}

#[test]
fn frametitle_color_wins_over_structure() {
    // The most specific `\setbeamercolor{frametitle}{fg=…}` wins over the structure color.
    let t = deck("\\setbeamercolor{structure}{fg=blue}\\setbeamercolor{frametitle}{fg=red}");
    assert!(
        t.contains("config-colors(primary: red)"),
        "frametitle color wins over structure; got:\n{t}"
    );
    assert!(
        !t.contains("primary: blue"),
        "structure color must not override the more-specific frametitle; got:\n{t}"
    );
}
