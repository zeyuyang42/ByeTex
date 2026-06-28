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
fn ref_followed_by_dot_label_text_does_not_glue() {
    // `\Cref{def:shape_regular}.ii` (corpus 2605.22159): the `.ii` is literal text
    // after the ref, but Typst's `@key` syntax greedily absorbs a `.` that is
    // followed by more label chars — yielding `@def:shape_regular.ii`, a dangling
    // reference to a label that doesn't exist. A separator must break the glue.
    let src = "\\begin{defn}\\label{def:shape_regular}\nx\n\\end{defn}\n\
        See \\Cref{def:shape_regular}.ii here.";
    let t = typst(src);
    assert!(
        !t.contains("@def:shape_regular.ii"),
        "the trailing `.ii` text must not glue onto the reference;\noutput:\n{t}"
    );
    assert!(
        t.contains("@def:shape_regular .ii") || t.contains("#ref(<def:shape_regular>)"),
        "the reference must resolve to `def:shape_regular`, with `.ii` kept separate;\noutput:\n{t}"
    );
}

#[test]
fn ref_followed_by_bare_period_is_unaffected() {
    // A `.` that is NOT followed by a label char is ordinary sentence
    // punctuation; Typst does not absorb it, so no separator must be inserted.
    let src = "\\begin{defn}\\label{def:foo_bar}\nx\n\\end{defn}\n\
        See \\Cref{def:foo_bar}.";
    let t = typst(src);
    assert!(
        t.contains("@def:foo_bar."),
        "a ref before a bare period must stay glued to the period (no spurious space);\noutput:\n{t}"
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
