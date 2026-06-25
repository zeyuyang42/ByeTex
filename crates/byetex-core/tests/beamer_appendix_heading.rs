//! In beamer, `\appendix` marks backup slides — frame titles stay unnumbered
//! (appendixnumberbeamer changes the *slide* number, not heading numbers).
//! ByeTex emitted article-style `#set heading(numbering: "A.1")`, which numbered
//! the appendix frame title (a level-2 heading) with a 0-valued level-1 counter,
//! producing a degenerate `-.1` prefix (e.g. `-.1 Backup slides`). The numbering
//! reset must be skipped for beamer. Found by the visual grader on gh-klb2-beamer.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn beamer_appendix_does_not_set_letter_numbering() {
    let src = r"\documentclass{beamer}\begin{document}\begin{frame}{Main}Hi.\end{frame}\appendix\begin{frame}{Backup slides}Extra.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("Backup slides"), "lost appendix frame; got:\n{t}");
    assert!(
        !t.contains(r#"numbering: "A.1""#),
        "beamer appendix wrongly set letter heading numbering; got:\n{t}"
    );
}

#[test]
fn article_appendix_still_sets_letter_numbering() {
    let src = r"\documentclass{article}\begin{document}\section{Body}\appendix\section{Extra}\end{document}";
    let t = typ(src);
    assert!(
        t.contains(r#"numbering: "A.1""#),
        "article appendix numbering regressed; got:\n{t}"
    );
}
