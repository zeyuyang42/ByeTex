//! Regression tests for \phantom / \hphantom / \vphantom in math mode.
//!
//! In Typst math, `hide(inner)` is invalid — `hide` is a content function,
//! not a math operator. The emitter must produce `#hide[$inner$]` instead.

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

#[test]
fn phantom_does_not_emit_bare_hide_call() {
    let out = convert_str(r"$a + \phantom{b} + c$");
    assert!(
        !out.typst.contains("hide(b)") && !out.typst.contains("hide(b )"),
        "bare hide(...) is invalid in Typst math, got: {}",
        out.typst
    );
}

#[test]
fn phantom_emits_hash_hide_bracket() {
    let out = convert_str(r"$a + \phantom{b} + c$");
    assert!(
        out.typst.contains("#hide["),
        "expected #hide[...] syntax, got: {}",
        out.typst
    );
}

#[test]
fn phantom_inline_math_compiles() {
    // Regression: \phantom{XYZ} must produce valid Typst (no 'hide' unknown variable)
    let out = convert_str(r"$A_{\phantom{0}1}$");
    assert!(
        !out.typst.contains("hide("),
        "paren form hide(...) must not appear in math, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("#hide["),
        "expected #hide[...], got: {}",
        out.typst
    );
}
