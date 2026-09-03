//! The FORM of the beamer frame number, which differs by theme.
//!
//! touying's metropolis defaults `footer-right` to
//! `slide-counter.display() + " / " + last-slide-number`. The LaTeX metropolis
//! theme prints the number ALONE in the bottom-right — truth carries a bare
//! frame number on 25 of 33 slides (gh-mtheme-demo), 22 of 26
//! (gh-bard-metropolis) and 10 of 15 (gh-klb2-beamer).
//!
//! An earlier version of this rule suppressed the footer outright, on a
//! measurement that only matched the "N / M" spelling and so reported truth as
//! having no counter at all. It has one; only the total is wrong.
//!
//! Madrid DOES print "N / M" — that is touying's default — so it is left alone.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = "\\documentclass{beamer}\n\\usetheme{metropolis}\n\\title{T}\n\
    \\begin{document}\n\\begin{frame}{One}\nBody.\n\\end{frame}\n\\end{document}";

#[test]
fn the_metropolis_theme_prints_a_bare_frame_number() {
    let t = typ(DECK);
    assert!(
        t.contains("metropolis-theme.with"),
        "expected the metropolis theme; got:\n{t}"
    );
    assert!(
        t.contains("footer-right: context utils.slide-counter.display()"),
        "metropolis prints the frame number alone; got:\n{t}"
    );
    assert!(
        !t.contains("last-slide-number"),
        "the \" / total\" half must be dropped; got:\n{t}"
    );
}

#[test]
fn the_rest_of_the_theme_configuration_survives() {
    // The suppression is an added argument, not a replacement for the ones that
    // carry the deck's identity.
    let t = typ(DECK);
    for want in ["aspect-ratio:", "config-page(", "config-info("] {
        assert!(t.contains(want), "{want} must survive; got:\n{t}");
    }
}

#[test]
fn a_non_beamer_document_is_untouched() {
    // The control: nothing about this reaches an ordinary article.
    let t = typ("\\documentclass{article}\n\\begin{document}\nx\n\\end{document}");
    assert!(!t.contains("footer-right"), "articles are unaffected; got:\n{t}");
}

#[test]
fn a_theme_that_numbers_its_frames_keeps_the_counter() {
    // `beamer-demo` uses `\usetheme{Madrid}`, whose footline carries a frame
    // number — its truth shows one on all 8 slides. Suppressing the counter
    // unconditionally took that away, which is why this rule is theme-aware.
    let t = typ(
        "\\documentclass{beamer}\n\\usetheme{Madrid}\n\\title{T}\n\\begin{document}\n\
         \\begin{frame}{One}\nBody.\n\\end{frame}\n\\end{document}",
    );
    assert!(
        !t.contains("footer-right:"),
        "Madrid's \"N / M\" IS touying's default; leave it alone; got:\n{t}"
    );
}

#[test]
fn a_deck_naming_no_theme_is_treated_as_metropolis() {
    // gh-klb2-beamer declares no `\usetheme` and its truth shows no counter;
    // metropolis is also what we render every deck as.
    let t = typ(
        "\\documentclass{beamer}\n\\title{T}\n\\begin{document}\n\
         \\begin{frame}{One}\nBody.\n\\end{frame}\n\\end{document}",
    );
    assert!(
        t.contains("footer-right: context utils.slide-counter.display()"),
        "no theme → metropolis form; got:\n{t}"
    );
}
