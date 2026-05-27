//! Regression tests for Bug #49: `\ensuremath` creating nested `$...$` in math.
//!
//! The macro-seed definition had body `$#1$`, which produced `$cal(M)$` inside
//! math mode — Typst "unclosed delimiter" error. The fix moves `\ensuremath` to
//! a mode-aware emitter handler:
//! - in math: render arg content directly (no `$` wrapping)
//! - in text: wrap in `$...$`

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

// ── Bug #49: no nested $$ when \ensuremath is used inside math mode ───────────

#[test]
fn ensuremath_in_math_mode_no_nested_dollar() {
    // \ensuremath inside an existing math environment must not add `$...$`
    let out = convert_str(r"$x = \ensuremath{y}$");
    assert!(
        !out.typst.contains("$$"),
        "nested $$ found in output: {}",
        out.typst
    );
    // Content must survive
    assert!(
        out.typst.contains('y') || out.typst.contains("y"),
        "content 'y' must survive: {}",
        out.typst
    );
}

#[test]
fn ensuremath_via_macro_in_math_no_nested_dollar() {
    // The canonical corpus failure: \cM defined as \ensuremath{\mathcal{M}},
    // then used inside math.
    let out = convert_str(r"\newcommand{\cM}{\ensuremath{\mathcal{M}}} $A = \cM$");
    assert!(
        !out.typst.contains("$$"),
        "nested $$ found in output: {}",
        out.typst
    );
    assert!(
        out.typst.contains("cal(M)") || out.typst.contains("cal("),
        "mathcal M must render: {}",
        out.typst
    );
}

// ── Positive: \ensuremath in text mode still wraps in $..$ ────────────────────

#[test]
fn ensuremath_in_text_mode_adds_math_wrapper() {
    let out = convert_str(r"Let \ensuremath{x} be a variable.");
    assert!(
        out.typst.contains("$x$") || out.typst.contains("$ x $"),
        "text-mode \\ensuremath must wrap in $: {}",
        out.typst
    );
}
