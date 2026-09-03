//! touying's metropolis theme prints a slide counter; LaTeX's does not.
//!
//! `metropolis-theme`'s store defaults `footer-right` to
//! `utils.slide-counter.display()`, so every slide gets an "N / M" footer. The
//! LaTeX theme these decks actually use prints nothing there.
//!
//! Measured across the corpus's metropolis-family decks — slides carrying an
//! "N / M" marker, truth vs ours before this change:
//!
//!   gh-mtheme-demo      truth 0/33   ours 27/33
//!   gh-bard-metropolis  truth 0/26   ours 21/25
//!   gh-klb2-beamer      truth 0/15   ours 11/15
//!
//! Not a blanket rule: `beamer-demo` uses `\usetheme{Madrid}`, whose footline
//! DOES carry a frame number, and its truth shows one on 8 of 8 slides. This
//! suppression is specific to the metropolis theme we emit.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = "\\documentclass{beamer}\n\\usetheme{metropolis}\n\\title{T}\n\
    \\begin{document}\n\\begin{frame}{One}\nBody.\n\\end{frame}\n\\end{document}";

#[test]
fn the_metropolis_theme_suppresses_the_slide_counter() {
    let t = typ(DECK);
    assert!(
        t.contains("metropolis-theme.with"),
        "expected the metropolis theme; got:\n{t}"
    );
    assert!(
        t.contains("footer-right: none"),
        "touying's default slide counter must be suppressed to match LaTeX; got:\n{t}"
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
        !t.contains("footer-right: none"),
        "Madrid numbers its frames; the counter must stay; got:\n{t}"
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
    assert!(t.contains("footer-right: none"), "no theme → no counter; got:\n{t}");
}
