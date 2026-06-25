//! `\begin{frame}[standout]{…}` is a metropolis "focus" slide — a full
//! dark-background slide with large centered text. ByeTex ignored the `[standout]`
//! option and rendered the body as an ordinary frame, so the distinctive styling
//! was lost (and the content merged onto a neighbouring slide). Map it to
//! touying-metropolis's `#focus-slide[…]`. Found by the visual grader on
//! gh-klb2-beamer.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn standout_frame_becomes_focus_slide() {
    let src = r"\documentclass{beamer}\usetheme{metropolis}\begin{document}\begin{frame}[standout]Standout Frame!\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("#focus-slide["), "standout frame not a focus-slide; got:\n{t}");
    assert!(t.contains("Standout Frame!"), "lost standout body; got:\n{t}");
}

#[test]
fn standout_with_plain_option_detected() {
    let src = r"\documentclass{beamer}\usetheme{metropolis}\begin{document}\begin{frame}[standout,plain]Focus.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("#focus-slide["), "standout,plain not detected; got:\n{t}");
}

#[test]
fn normal_frame_is_not_a_focus_slide() {
    let src = r"\documentclass{beamer}\usetheme{metropolis}\begin{document}\begin{frame}{Title}Regular body.\end{frame}\end{document}";
    let t = typ(src);
    assert!(!t.contains("#focus-slide["), "normal frame wrongly made a focus-slide; got:\n{t}");
    assert!(t.contains("== Title"), "normal frame lost its title; got:\n{t}");
}
