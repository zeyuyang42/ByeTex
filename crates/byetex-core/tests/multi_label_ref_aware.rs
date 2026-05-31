//! Ref-aware multi-label: `\section{X}\label{a}\label{b}` gives the section
//! two aliases, but Typst keeps only one label per element. So ByeTex attaches
//! whichever alias is actually `\ref`'d — otherwise a reference to a non-first
//! alias dangles (`label <b> does not exist`). When none is referenced, the
//! first is kept (unchanged behavior).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn referenced_alias_is_attached_not_first() {
    let src =
        "\\section{Related work}\\label{sec:rw}\\label{app:rw}\n\nText.\n\nSee \\ref{app:rw}.";
    let t = typ(src);
    assert!(
        t.contains("<app:rw>"),
        "the referenced second alias must be attached; got:\n{t}"
    );
    // @app:rw must resolve to that heading.
    assert!(
        t.contains("@app:rw"),
        "the ref itself should emit @app:rw; got:\n{t}"
    );
}

#[test]
fn first_label_kept_when_none_referenced() {
    // No \ref → keep the first label (existing behavior).
    let src = "\\section{X}\\label{a}\\label{b}\nText.";
    let t = typ(src);
    assert!(
        t.contains("<a>"),
        "first label kept when none referenced; got:\n{t}"
    );
}

#[test]
fn first_referenced_alias_wins_when_several_referenced() {
    // Typst can hold only one label; when multiple aliases are referenced,
    // pick the first label that is referenced (deterministic, and at least
    // that reference resolves).
    let src = "\\section{X}\\label{a}\\label{b}\n\nSee \\ref{b} and \\ref{a}.";
    let t = typ(src);
    // `a` is the first label AND is referenced → it's attached.
    assert!(
        t.contains("= X <a>"),
        "first referenced alias attached; got:\n{t}"
    );
}

#[test]
fn cref_and_eqref_also_count_as_references() {
    let src = "\\section{X}\\label{a}\\label{b}\n\nSee \\cref{b}.";
    let t = typ(src);
    assert!(
        t.contains("<b>"),
        "\\cref target must be treated as referenced; got:\n{t}"
    );
}
