//! Beamer title slide / `\frame{…}` command form (B3). `\frame{\titlepage}` and the
//! short `\frame{content}` form used to be dropped (`unsupported_command`) — losing
//! short-form slide content and warning on the title slide. Now `\frame{…}` renders as
//! a slide and `\titlepage` is a no-op (the title block is auto-emitted at the top).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = r#"\documentclass{beamer}
\title{My Talk}
\author{Jane Doe}
\begin{document}
\frame{\titlepage}
\begin{frame}{First}
Content one.
\end{frame}
\frame{Short form slide.}
\end{document}"#;

#[test]
fn frame_command_form_content_is_kept() {
    let t = typ(DECK);
    assert!(t.contains("Short form slide."), "\\frame{{…}} short form kept; got:\n{t}");
}

#[test]
fn titlepage_does_not_leak_and_title_renders() {
    let t = typ(DECK);
    assert!(t.contains("My Talk"), "title block still rendered");
    assert!(!t.contains("titlepage"), "\\titlepage must not leak as text; got:\n{t}");
}

#[test]
fn titlepage_frame_makes_no_blank_slide() {
    // `\frame{\titlepage}` renders to nothing extra (title is at the top), so it must
    // not emit a stray pagebreak with empty content. Exactly the content frames break.
    let t = typ(DECK);
    // Two real slides after the title: "First" and the short-form slide.
    assert_eq!(t.matches("#pagebreak").count(), 2, "one pagebreak per content slide; got:\n{t}");
}

#[test]
fn non_beamer_frame_command_unaffected() {
    let t = typ("\\documentclass{article}\\begin{document}\\frame{x}\\end{document}");
    assert!(!t.contains("#pagebreak"), "non-beamer \\frame must not be slide-styled; got:\n{t}");
}
