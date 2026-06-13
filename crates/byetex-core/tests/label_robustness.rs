//! Two label-robustness fixes (corpus 2605.31579, 2605.31345):
//!
//! 1. A label key with spaces/`<` (`\label{lemma:bispectrum < A3f}`) sanitizes
//!    to a hyphen run (`lemma:bispectrum---A3f`); `post_process_typography` then
//!    turns `---` into an em-dash INSIDE the `<…>` token → Typst "unclosed
//!    label". Fix: `sanitize_label_key` collapses `-` runs to a single `-`, so
//!    no `--`/`---` ever reaches the dash pass. Def and ref stay consistent.
//!
//! 2. The same label key defined twice (LaTeX warns; Typst hard-errors
//!    "label occurs multiple times"). Fix: global dedup — emit each `<key>`
//!    only on first use; later definitions are skipped (refs resolve to the
//!    first). 2605.31345 duplicated `\label{ssec:comparison}`.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn label_with_spaces_and_angle_collapses_to_single_hyphen() {
    let src = "\\begin{align}\\label{lemma:bispectrum < A3f}\nx = 1\n\\end{align}\n\
        See \\ref{lemma:bispectrum < A3f}.";
    let t = typst(src);
    assert!(
        t.contains("<lemma:bispectrum-A3f>"),
        "label must collapse to a single hyphen;\noutput:\n{t}"
    );
    assert!(
        t.contains("@lemma:bispectrum-A3f") || t.contains("#ref(<lemma:bispectrum-A3f>)"),
        "the reference must use the same collapsed key;\noutput:\n{t}"
    );
    // The bug: a hyphen run becomes an em-dash (U+2014) / en-dash (U+2013) and
    // breaks the label token. Neither must appear.
    assert!(
        !t.contains('\u{2014}') && !t.contains('\u{2013}'),
        "no em/en-dash may appear in the output (would break the label);\noutput:\n{t}"
    );
    assert!(
        !t.contains("---") && !t.contains("bispectrum--"),
        "no multi-hyphen run may remain in the label;\noutput:\n{t}"
    );
}

#[test]
fn duplicate_label_is_emitted_only_once() {
    let src = "\\subsection{First}\\label{ssec:comparison}\n\
        Body one.\n\
        \\subsection{Second}\\label{ssec:comparison}\n\
        Body two. See \\ref{ssec:comparison}.";
    let t = typst(src);
    let n = t.matches("<ssec:comparison>").count();
    assert_eq!(
        n, 1,
        "a duplicated label must be emitted exactly once (Typst rejects dups);\n\
         found {n} occurrences;\noutput:\n{t}"
    );
    // The reference must still resolve to the surviving label.
    assert!(
        t.contains("@ssec:comparison"),
        "the reference must still be emitted;\noutput:\n{t}"
    );
}
