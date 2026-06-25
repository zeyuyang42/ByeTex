//! `\usetheme[progressbar=frametitle]{metropolis}` (and friends) leaked the
//! `[options]{name}` as slide body text. The theme command is presentation-only
//! and dropped, but tree-sitter parses the *command* as a bare `generic_command`
//! (no children) with the `[opts]` and `{name}` as following siblings, so the
//! `node.end_byte()` drop didn't cover them. Found by the visual grader on
//! gh-klb2-beamer (raw `[progressbar=frametitle]metropolis` on slide 2).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn usetheme_with_options_does_not_leak() {
    let src = r"\documentclass{beamer}
\usetheme[progressbar=frametitle]{metropolis}
\begin{document}
\begin{frame}
Body.
\end{frame}
\end{document}";
    let t = typ(src);
    assert!(t.contains("Body."), "lost body; got:\n{t}");
    assert!(!t.contains("progressbar"), "leaked theme option; got:\n{t}");
    assert!(!t.contains("frametitle"), "leaked theme option; got:\n{t}");
    // The theme name string must not leak as body text either.
    assert!(
        !t.lines().any(|l| l.trim() == "metropolis" || l.trim() == r"\[progressbar=frametitle\]metropolis"),
        "leaked theme name as body text; got:\n{t}"
    );
}

#[test]
fn usetheme_without_options_still_dropped() {
    let src = r"\documentclass{beamer}
\usetheme{Madrid}
\begin{document}
\begin{frame}
Hi.
\end{frame}
\end{document}";
    let t = typ(src);
    assert!(t.contains("Hi."), "lost body; got:\n{t}");
    assert!(!t.lines().any(|l| l.trim() == "Madrid"), "leaked theme name; got:\n{t}");
}
