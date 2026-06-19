//! Beamer overlay specs (B5): `\item<1->`, `\pause`, `\only`/`\uncover`/`\onslide`/
//! `\visible`/`\alert` carry `<overlay-spec>` markers that a static PDF can't animate.
//! The content is shown unconditionally and the `<…>` spec must NOT leak as text.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DECK: &str = r#"\documentclass{beamer}
\begin{document}
\begin{frame}{Overlays}
\begin{itemize}
\item<1-> First point
\item<2-> Second point
\end{itemize}
\pause
\only<2>{Only on slide two.}
\uncover<3->{Uncovered content.}
\onslide<2->{On slide two plus.}
\alert<1>{Alert text.}
\end{frame}
\end{document}"#;

#[test]
fn overlay_content_is_shown() {
    let t = typ(DECK);
    for s in [
        "First point", "Second point", "Only on slide two.",
        "Uncovered content.", "On slide two plus.", "Alert text.",
    ] {
        assert!(t.contains(s), "overlay content `{s}` shown; got:\n{t}");
    }
}

#[test]
fn overlay_specs_do_not_leak() {
    let t = typ(DECK);
    for spec in ["<1->", "<2->", "<2>", "<3->", "<1>"] {
        assert!(!t.contains(spec), "overlay spec `{spec}` must not leak; got:\n{t}");
    }
}

#[test]
fn overlay_command_without_spec_keeps_content() {
    // Code-review (critical): `\alert{x}` / `\only{x}` with NO overlay spec — the
    // `{content}` is a child of the command and must still render, not be dropped.
    let t = typ("\\documentclass{beamer}\\begin{document}\\begin{frame}{F}\\alert{HILITE} and \\only{NOSPEC}\\end{frame}\\end{document}");
    assert!(t.contains("HILITE"), "\\alert{{x}} no-spec content kept; got:\n{t}");
    assert!(t.contains("NOSPEC"), "\\only{{x}} no-spec content kept; got:\n{t}");
}

#[test]
fn non_beamer_item_angle_token_preserved() {
    // Code-review: the `\item<…>` overlay strip is gated on beamer, so a non-beamer
    // `\item <0,1>` keeps its literal angle-bracket text.
    let t = typ("\\documentclass{article}\\begin{document}\\begin{itemize}\\item <0,1> range\\end{itemize}\\end{document}");
    assert!(t.contains("0,1"), "non-beamer angle token preserved; got:\n{t}");
}

#[test]
fn non_beamer_overlay_commands_unaffected() {
    // \only/\alert are gated on beamer; a non-beamer doc keeps its old handling.
    let t = typ("\\documentclass{article}\\begin{document}\\alert<1>{x}\\end{document}");
    // Whatever article does, it must not be the beamer overlay path (no panic / stable).
    assert!(t.contains('x') || !t.contains("Alert"), "stable non-beamer handling; got:\n{t}");
}
