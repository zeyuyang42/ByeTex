//! Foundational beamer support: ByeTex used to drop every `frame` environment
//! (`unsupported_environment`, all slide content lost). Now the `beamer` class is
//! detected and each `frame` renders its title + body so the content survives.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = r#"\documentclass{beamer}
\title{A Talk}
\author{Jane Doe}
\begin{document}
\begin{frame}{First Slide}
  \begin{itemize}
    \item Point one
    \item Point two
  \end{itemize}
\end{frame}
\begin{frame}
  \frametitle{Second Slide}
  Body of the second slide.
\end{frame}
\end{document}"#;

#[test]
fn frame_body_content_is_kept() {
    let t = typ(DECK);
    assert!(t.contains("Point one"), "frame list content kept; got:\n{t}");
    assert!(t.contains("Point two"), "frame list content kept");
    assert!(t.contains("Body of the second slide."), "second frame body kept");
}

#[test]
fn frame_titles_render() {
    let t = typ(DECK);
    assert!(t.contains("First Slide"), "frame {{title}} arg rendered; got:\n{t}");
    assert!(t.contains("Second Slide"), "\\frametitle rendered");
}

#[test]
fn frames_are_separate_slides() {
    // touying: each frame title is a level-2 heading (`== Title`); metropolis starts
    // a fresh slide per `==`, so no manual `#pagebreak` is emitted.
    let t = typ(DECK);
    assert!(t.contains("== First Slide"), "first frame is a `==` slide; got:\n{t}");
    assert!(t.contains("== Second Slide"), "second frame (\\frametitle) is a `==` slide; got:\n{t}");
    assert!(!t.contains("pagebreak"), "touying slides use `==`, not pagebreaks; got:\n{t}");
}

#[test]
fn body_group_on_new_line_is_not_a_title() {
    // Code-review: a frame whose body opens with a `{...}` group on a NEW line must
    // keep it as body, not consume it as the slide title.
    let t = typ("\\documentclass{beamer}\\begin{document}\n\\begin{frame}\n{\\bf grouped body}\nrest\n\\end{frame}\n\\end{document}");
    assert!(t.contains("grouped body"), "body group kept; got:\n{t}");
    // It must not have been emitted as a bold 1.2em title.
    assert!(!t.contains("size: 1.2em, weight: \"bold\")[grouped body]"),
        "body group wrongly used as title; got:\n{t}");
}

#[test]
fn frame_subtitle_is_kept() {
    // `\begin{frame}{Title}{Subtitle}` — the subtitle must not leak as raw body.
    let t = typ("\\documentclass{beamer}\\begin{document}\n\\begin{frame}{The Title}{The Subtitle}\nBody.\n\\end{frame}\n\\end{document}");
    assert!(t.contains("The Title"), "title kept");
    assert!(t.contains("The Subtitle"), "subtitle kept; got:\n{t}");
    // Subtitle rendered as styled text, not leaked as a bare run before "Body".
    assert!(t.contains("[The Subtitle]"), "subtitle styled; got:\n{t}");
}

#[test]
fn non_beamer_frame_env_is_not_slide_styled() {
    // The `frame` env arm is gated on the beamer class — a non-beamer doc's `frame`
    // must not get slide pagebreaks.
    let t = typ("\\documentclass{article}\\begin{document}\n\\begin{frame}\nx\n\\end{frame}\n\\end{document}");
    assert!(!t.contains("pagebreak"), "non-beamer frame must not be slide-styled; got:\n{t}");
}
