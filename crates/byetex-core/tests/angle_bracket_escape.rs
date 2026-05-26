//! Regression tests for Bug #48: angle brackets in text emitted as-is,
//! which Typst interprets as label syntax `<key>`.
//!
//! A Typst label key must match `[a-zA-Z0-9_:.-]+`. When the text between
//! `<` and `>` contains other characters (e.g. `@`, `/`, `\`), the `<` must
//! be escaped as `\<` so Typst does not try to parse it as a label.

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

// ── Stray angle brackets that are NOT valid labels ────────────────────────────

#[test]
fn angle_bracket_with_at_sign_is_escaped() {
    // `<email@domain.com>` — `@` is not a valid label char
    let out = convert_str(r"See \texttt{user@host} or contact <admin@example.com> directly.");
    assert!(
        out.typst.contains("\\<admin"),
        "expected escaped \\<admin in output: {}",
        out.typst
    );
    // The `<` must be preceded by `\` — not a bare `<`
    assert!(
        !out.typst.contains(" <admin"),
        "bare (unescaped) angle bracket must not appear: {}",
        out.typst
    );
}

#[test]
fn angle_bracket_with_slash_is_escaped() {
    // `<http://url>` — `/` is not a valid label char
    let out2 = convert_str(r"Go to <http://example.com> for details.");
    assert!(
        out2.typst.contains("\\<http://"),
        "expected escaped \\<http:// in output: {}",
        out2.typst
    );
    assert!(
        !out2.typst.contains(" <http://"),
        "bare angle bracket must not appear: {}",
        out2.typst
    );
}

// ── Valid label keys must NOT be escaped ──────────────────────────────────────

#[test]
fn valid_label_from_label_command_not_escaped() {
    // `\label{sec:intro}` should emit `<sec:intro>` — valid label chars only
    let out = convert_str(r"= Introduction \label{sec:intro}");
    assert!(
        out.typst.contains("<sec:intro>"),
        "valid label should not be escaped: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\<sec:intro>"),
        "valid label must not be double-escaped: {}",
        out.typst
    );
}

#[test]
fn valid_label_with_dots_and_dashes_not_escaped() {
    let out = convert_str(r"Some text \label{fig.a-b} end");
    assert!(
        out.typst.contains("<fig.a-b>"),
        "label with dots/dashes should be valid: {}",
        out.typst
    );
}

// ── No escaping of angle brackets already inside math ─────────────────────────

#[test]
fn angle_brackets_in_math_not_escaped() {
    // In math mode, `<` and `>` are comparison operators — must not be touched
    let out = convert_str(r"We have $x < y > z$ always.");
    assert!(
        out.typst.contains("x < y > z") || out.typst.contains("x<y>z") || out.typst.contains("x < y"),
        "math comparison operators should survive: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\<") || !out.typst.contains("x"),
        "no spurious escaping inside math mode: {}",
        out.typst
    );
}
