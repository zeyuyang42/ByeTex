//! Regression tests for Bug #46: `\label` key sanitization in the body emitter.
//!
//! `\label{foo,bar}` must produce `<foo-bar>` (comma → `-`), not `<foo,bar>`
//! which is invalid Typst syntax.

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

// ── Bug #46: label key sanitization ──────────────────────────────────────────

#[test]
fn label_with_comma_is_sanitized() {
    let out = convert_str(r"Some text \label{fig:foo,bar} more text");
    assert!(
        !out.typst.contains("<fig:foo,bar>"),
        "unsanitized label found in output: {}",
        out.typst
    );
    assert!(
        out.typst.contains("<fig:foo-bar>"),
        "expected sanitized label <fig:foo-bar> in output: {}",
        out.typst
    );
}

#[test]
fn label_with_spaces_is_sanitized() {
    let out = convert_str(r"Some text \label{my label} end");
    assert!(
        !out.typst.contains("<my label>"),
        "unsanitized label with space in output: {}",
        out.typst
    );
    assert!(
        out.typst.contains("<my-label>"),
        "expected sanitized label <my-label> in output: {}",
        out.typst
    );
}

#[test]
fn label_with_normal_key_unchanged() {
    let out = convert_str(r"Some text \label{sec:intro} end");
    assert!(
        out.typst.contains("<sec:intro>"),
        "clean label should be preserved as-is: {}",
        out.typst
    );
}

#[test]
fn label_in_math_env_with_comma_sanitized() {
    let out = convert_str(
        r"\begin{equation} x = y \label{eq:foo,bar} \end{equation}",
    );
    assert!(
        !out.typst.contains("<eq:foo,bar>"),
        "unsanitized math-env label in output: {}",
        out.typst
    );
    assert!(
        out.typst.contains("<eq:foo-bar>"),
        "expected sanitized label <eq:foo-bar> in output: {}",
        out.typst
    );
}
