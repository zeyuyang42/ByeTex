//! Regression tests for Bug #47: commas inside `emit_math_wrap` functions.
//!
//! In Typst math, `func(a,b)` passes *two* arguments — comma is an arg
//! separator — so `\mathtt{inv,h}` → `mono(inv,h)` is invalid.  When the
//! rendered content of a single-arg math wrapper contains a comma, the
//! emitter must switch to content-block syntax `func[inner]` where commas
//! are just content.

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

// ── Bug #47: mathtt with comma ────────────────────────────────────────────────

#[test]
fn mathtt_with_comma_does_not_use_paren_syntax() {
    let out = convert_str(r"$\mathtt{inv,h}$");
    // `mono(inv,h)` would be invalid Typst — comma is arg separator
    assert!(
        !out.typst.contains("mono(inv,h)"),
        "invalid paren syntax in output: {}",
        out.typst
    );
}

#[test]
fn mathtt_with_comma_uses_bracket_syntax() {
    let out = convert_str(r"$\mathtt{inv,h}$");
    assert!(
        out.typst.contains("mono[inv,h]") || out.typst.contains("mono[i n v,h]"),
        "expected bracket syntax for comma content, got: {}",
        out.typst
    );
}

// ── Bug #47: overline with comma ──────────────────────────────────────────────

#[test]
fn overline_with_comma_does_not_use_paren_syntax() {
    let out = convert_str(r"$\overline{a,b}$");
    assert!(
        !out.typst.contains("overline(a,b)") && !out.typst.contains("overline(a , b)"),
        "invalid paren syntax in output: {}",
        out.typst
    );
}

#[test]
fn overline_with_comma_uses_bracket_syntax() {
    let out = convert_str(r"$\overline{a,b}$");
    assert!(
        out.typst.contains("overline["),
        "expected bracket syntax for comma content, got: {}",
        out.typst
    );
}

// ── Negative: no regression for single-arg (no comma) ───────────────────────

#[test]
fn overline_without_comma_uses_paren_syntax() {
    let out = convert_str(r"$\overline{x}$");
    assert!(
        out.typst.contains("overline(x)"),
        "single-arg overline should keep paren syntax: {}",
        out.typst
    );
}

#[test]
fn mathtt_without_comma_uses_paren_syntax() {
    let out = convert_str(r"$\mathtt{abc}$");
    assert!(
        out.typst.contains("mono(") || out.typst.contains("mono["),
        "mathtt without comma should produce mono(...): {}",
        out.typst
    );
    assert!(
        !out.typst.contains("mono(a b c,"),
        "no spurious comma in output: {}",
        out.typst
    );
}
