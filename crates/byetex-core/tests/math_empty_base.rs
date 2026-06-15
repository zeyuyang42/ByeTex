//! Expanded-corpus compile-blocker (2605.31203): a superscript with no base
//! atom — `\mu(^{233}\mathrm{U})`, the isotope/prescript idiom where `^{233}`
//! immediately follows an opening `(` — emitted a bare `(^(233)...)`. Typst's
//! `^` requires a left operand, so this is `error: unexpected hat` → compile
//! failure. byetex already inserts an empty `""` base right after `$` (a
//! floating `{}^{a}` footnote marker); the same repair must fire after an
//! opening delimiter `(`/`[`/`{`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn superscript_after_open_paren_gets_empty_base() {
    // The 2605.31203 shape: `^{233}` directly after `(`.
    let t = typ("$\\mu(^{233}\\mathrm{U})$");
    // The caret must NOT directly follow the opening paren (Typst rejects `(^`).
    assert!(
        !t.contains("(^"),
        "a `^` after `(` needs an empty base, not a bare caret; got:\n{t}"
    );
    // It should be repaired with an empty-string base: `(""^(233)...`.
    assert!(
        t.contains("(\"\"^"),
        "expected an empty `\"\"` base before the superscript; got:\n{t}"
    );
}

#[test]
fn subscript_after_open_paren_gets_empty_base() {
    let t = typ("$f(_{i}x)$");
    assert!(
        !t.contains("(_"),
        "a `_` after `(` needs an empty base, not a bare underscore; got:\n{t}"
    );
}

#[test]
fn normal_superscript_with_base_is_untouched() {
    // Regression guard: a real base before `^` must stay attached (no spurious
    // empty base injected).
    let t = typ("$x^2 + (a+b)^2$");
    assert!(
        t.contains("x^2"),
        "base `x` must keep its superscript; got:\n{t}"
    );
    assert!(
        !t.contains("\"\"^"),
        "no empty base should be injected when a real base exists; got:\n{t}"
    );
}
