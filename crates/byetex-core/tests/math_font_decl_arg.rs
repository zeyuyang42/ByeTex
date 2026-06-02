//! Expanded-corpus compile-blocker (2605.31306): `\frac{\rm{d} {\mathbb Q}}{...}`
//! uses the old font-switch declaration `\rm` immediately followed by a braced
//! group (`\rm{d}`). tree-sitter parses `{d}` as a CHILD of the `\rm`
//! generic_command, but `emit_math_node_slice` only wrapped the following
//! SIBLINGS in `upright(...)` — so `{d}` was dropped, leaving an empty
//! `upright()` (→ Typst `missing argument: body` → compile failure).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn rm_decl_with_braced_arg_keeps_content() {
    let t = typ("$\\frac{\\rm{d} x}{\\rm{d} y}$");
    // No empty `upright()` may be emitted.
    assert!(
        !t.contains("upright()"),
        "an empty upright() means the \\rm argument was dropped; got:\n{t}"
    );
    // The differential `d` (the `\rm{d}` group) must survive in the numerator.
    assert!(
        t.contains("upright(d"),
        "the `d` from `\\rm{{d}}` must be kept inside upright(...); got:\n{t}"
    );
}

#[test]
fn rm_decl_with_two_groups_does_not_fuse() {
    // The 2605.31306 shape: `\rm{d} {\mathbb Q}` — `\rm` absorbs BOTH groups as
    // children. They must be space-separated so `d` and `bb(Q)` don't fuse into
    // `dbb` (Typst `unknown variable: dbb`).
    let t = typ("$\\frac{\\rm{d} {\\mathbb Q}}{\\rm{d} {\\mathbb P}}$");
    assert!(
        !t.contains("dbb"),
        "the differential `d` and `bb(Q)` must not fuse; got:\n{t}"
    );
    assert!(
        t.contains("upright(d bb(Q))"),
        "expected `upright(d bb(Q))`; got:\n{t}"
    );
}

#[test]
fn rm_decl_with_multiletter_word_is_quoted() {
    // The 2605.31510 shape: `\rm{db2mag}(\cdot)` — a function name. It must be
    // quoted as one token, not split into atoms (`d b 2mag` → `2mag` is an
    // invalid Typst variable).
    let t = typ("$\\rm{db2mag}(x)$");
    assert!(
        t.contains("\"db2mag\""),
        "a multi-letter \\rm word must be quoted; got:\n{t}"
    );
    // The broken split was `d b 2mag` (spaced atoms) — guard against it.
    assert!(
        !t.contains("b 2mag"),
        "the word must not be split into atoms; got:\n{t}"
    );
}

#[test]
fn bare_rm_decl_still_scopes_siblings() {
    // Regression guard: the bare-declaration form `{\rm d}` (no braced arg)
    // must still wrap the following sibling.
    let t = typ("${\\rm d}z$");
    assert!(
        t.contains("upright(d)"),
        "a bare `\\rm` must still wrap its sibling; got:\n{t}"
    );
}
