//! A beamer `\begin{frame}` with NO title (no `{title}` arg and no `\frametitle`)
//! is still its own slide. ByeTex emitted no `==` heading for it, so its body
//! flowed inline and MERGED onto the previous slide (V9: gh-klb2 "Frame without a
//! title" merged onto Motivation, dropping the deck from 15→14 slides). A
//! titleless frame must force a slide boundary.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn titleless_frame_forces_a_slide_boundary() {
    let src = r"\documentclass{beamer}\usetheme{metropolis}\begin{document}\begin{frame}{Motivation}Mot body.\end{frame}\begin{frame}No title here.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("== Motivation"), "lost titled frame; got:\n{t}");
    assert!(t.contains("No title here."), "lost titleless body; got:\n{t}");
    // A slide boundary must separate the titleless frame from the previous slide.
    assert!(
        t.contains("#pagebreak(weak: true)"),
        "titleless frame did not force a slide boundary; got:\n{t}"
    );
    // The boundary must sit BETWEEN the two frames' bodies (not before Motivation).
    let mot = t.find("Mot body.").unwrap();
    let brk = t.find("#pagebreak(weak: true)").unwrap();
    let titleless = t.find("No title here.").unwrap();
    assert!(mot < brk && brk < titleless, "boundary misplaced; got:\n{t}");
}

#[test]
fn titled_frame_needs_no_extra_boundary() {
    // Two titled frames: the `== Title` headings already create boundaries.
    let src = r"\documentclass{beamer}\usetheme{metropolis}\begin{document}\begin{frame}{One}A.\end{frame}\begin{frame}{Two}B.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("== One") && t.contains("== Two"), "lost titles; got:\n{t}");
    assert!(!t.contains("#pagebreak(weak: true)"), "titled frames got a spurious boundary; got:\n{t}");
}

#[test]
fn frametitle_command_frame_needs_no_extra_boundary() {
    // `\frametitle` in the body emits its own `==` heading — no extra boundary.
    let src = r"\documentclass{beamer}\usetheme{metropolis}\begin{document}\begin{frame}\frametitle{Body title}X.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("== Body title"), "frametitle not a heading; got:\n{t}");
    assert!(!t.contains("#pagebreak(weak: true)"), "frametitle frame got a spurious boundary; got:\n{t}");
}
