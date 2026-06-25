//! In touying-metropolis the accent (progress bar, section-divider rule,
//! `\alert`) is the `primary` colour — orange by default. A beamer deck that
//! customises the accent via `\setbeamercolor{alerted text}{fg=…}` (metropolis's
//! idiom) was ignored, so the accent stayed orange instead of the deck's colour.
//! Detect it and feed it to touying `config-colors(primary: …)`. Found by the
//! visual grader on gh-klb2-beamer (teal accent rendered orange).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn alerted_text_color_drives_touying_primary() {
    let src = r"\documentclass{beamer}\usetheme{metropolis}\definecolor{accent}{HTML}{7EBDC2}\setbeamercolor{alerted text}{fg=accent}\begin{document}\begin{frame}{F}Body.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("config-colors(primary:"), "accent not mapped to primary; got:\n{t}");
    assert!(
        t.contains("rgb(126, 189, 194)"),
        "deck accent colour not used; got:\n{t}"
    );
}

#[test]
fn no_accent_means_no_color_override() {
    let src = r"\documentclass{beamer}\usetheme{metropolis}\begin{document}\begin{frame}{F}Body.\end{frame}\end{document}";
    let t = typ(src);
    assert!(!t.contains("config-colors("), "spurious color override; got:\n{t}");
}

#[test]
fn structure_color_still_maps_to_primary() {
    let src = r"\documentclass{beamer}\usetheme{metropolis}\definecolor{acc}{HTML}{123456}\setbeamercolor{structure}{fg=acc}\begin{document}\begin{frame}{F}Body.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("config-colors(primary:"), "structure color regressed; got:\n{t}");
    assert!(t.contains("rgb(18, 52, 86)"), "structure color not used; got:\n{t}");
}
