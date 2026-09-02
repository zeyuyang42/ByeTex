//! `>{\bfseries}` and friends: an array-package column decorator whose body is a
//! FONT declaration, not an alignment one.
//!
//! `>{...}` prepends its content to every cell of the column that follows. The
//! alignment verbs (`\centering`, `\raggedright`, ...) were already honoured; a
//! font declaration was dropped, so an entire column silently lost its styling.
//! Verified against a control: `{>{\bfseries}l c}` produced output BYTE-IDENTICAL
//! to plain `{l c}`.
//!
//! The repo's own gap audit (`scripts/fidelity_audit.py`) ranks this as the only
//! open silent gap of the twelve it tracks: 18 papers, 116 occurrences — though
//! most of those are `@{}` spacing, and the font subset is ~12.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn table_of(spec: &str) -> String {
    typ(&format!(
        "\\begin{{tabular}}{{{spec}}}\na & b \\\\\nc & d \\\\\n\\end{{tabular}}"
    ))
}

#[test]
fn bfseries_decorator_bolds_its_column() {
    let out = table_of(">{\\bfseries}l c");
    assert!(
        out.contains("[#strong[a]], [b],"),
        "only the decorated column is bold; got:\n{out}"
    );
    assert!(
        out.contains("[#strong[c]], [d],"),
        "every row of that column is bold; got:\n{out}"
    );
}

#[test]
fn itshape_decorator_italicises_its_column() {
    let out = table_of(">{\\itshape}l c");
    assert!(out.contains("[#emph[a]], [b],"), "italic column; got:\n{out}");
}

#[test]
fn the_decorator_applies_to_the_following_column_only() {
    // `>{...}` binds to the column AFTER it — a second column must be untouched.
    let out = table_of("l >{\\bfseries}c");
    assert!(
        out.contains("[a], [#strong[b]],"),
        "the decorator binds to the FOLLOWING column; got:\n{out}"
    );
}

#[test]
fn an_alignment_decorator_still_only_sets_alignment() {
    // The control for the existing behaviour: `\centering` must keep producing an
    // alignment and must NOT start wrapping cells.
    let out = table_of(">{\\centering\\arraybackslash}p{2cm} l");
    assert!(out.contains("align: (center, left)"), "alignment preserved; got:\n{out}");
    assert!(
        !out.contains("#strong[") && !out.contains("#emph["),
        "an alignment decorator must not style cells; got:\n{out}"
    );
}

#[test]
fn a_plain_spec_is_unchanged() {
    // The other control: no decorator, no wrapping, byte-identical to before.
    let out = table_of("l c");
    assert!(
        !out.contains("#strong[") && !out.contains("#emph["),
        "plain columns must not be styled; got:\n{out}"
    );
}
