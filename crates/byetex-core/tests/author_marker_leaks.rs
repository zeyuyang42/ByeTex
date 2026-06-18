//! The author-block parser (`parse_authors`/`parse_one_author`) stripped
//! `\textsuperscript`/`\thanks`/etc. but NOT footnote-marker commands, so
//! `\author{\textbf{Yankai Lin\textsuperscript{1}\footnotemark[1]}}` leaked the
//! optional `[1]` as literal `\[1\]` next to the name (dogfood: recurring author-
//! block leakage, corpus 2606.12397 / 2605.22765). The markers (and their optional
//! `[N]`) are now stripped from author names.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn footnotemark_in_author_does_not_leak_bracket() {
    let t = typ(
        "\\documentclass{article}\\title{T}\\author{\\textbf{Yankai Lin\\textsuperscript{1}\\footnotemark[1]}}\\begin{document}\\maketitle Body.\\end{document}",
    );
    assert!(t.contains("Yankai Lin"), "author name kept; got:\n{t}");
    assert!(
        !t.contains("\\[1\\]") && !t.contains("[1\\]"),
        "the \\footnotemark[1] optional arg must not leak as literal text; got:\n{t}"
    );
}

#[test]
fn blfootnote_in_author_does_not_leak() {
    let t = typ(
        "\\documentclass{article}\\title{T}\\author{Alice\\blfootnote{Equal contribution}}\\begin{document}\\maketitle Body.\\end{document}",
    );
    assert!(t.contains("Alice"), "author name kept; got:\n{t}");
    assert!(!t.contains("blfootnote"), "\\blfootnote must not leak; got:\n{t}");
}

#[test]
fn plain_author_still_clean() {
    // Regression: an ordinary author with a superscript still renders.
    let t = typ(
        "\\documentclass{article}\\title{T}\\author{Bob\\textsuperscript{2}}\\begin{document}\\maketitle Body.\\end{document}",
    );
    assert!(t.contains("Bob"), "author name kept; got:\n{t}");
}
