//! `\begin{cases}` must not render literal square brackets.
//!
//! Found by the dogfood loop (2605.22728, 2026-08-14) — and notably NOT by any
//! tool: it compiles cleanly, so neither `warnings.json` nor `byetex diagnose`
//! saw it. The agent caught it only by looking at the rendered page.
//!
//! The emitter wrapped each row in `[...]` to stop internal commas being read as
//! `cases()` argument separators. That works — but in Typst MATH mode `[` and `]`
//! are literal bracket glyphs, not a content block, so every case row rendered as
//! `[1 quad "if" x > 0,]`, brackets and all. 22 corpus papers / 123 occurrences.
//!
//! The fix keeps the comma protection but drops the brackets: a TOP-LEVEL comma
//! inside a row becomes the quoted string `","` (the same idiom
//! `escape_paren_semicolons` already uses for `;`), while commas nested inside
//! `frac(a, b)` are left alone — Typst's own paren nesting already protects those.
//! The `&` column separator is also kept as Typst's `&` rather than being
//! flattened to `quad`, so the value/condition columns align like LaTeX's.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    )
    .typst
}

/// The text between the outermost `cases(` and its closing `)`.
fn cases_inner(out: &str) -> String {
    out.split_once("cases(")
        .and_then(|(_, rest)| rest.rsplit_once(')'))
        .map(|(inner, _)| inner.to_string())
        .unwrap_or_default()
}

const SIMPLE: &str = r"$f(x) = \begin{cases} 1 & \text{if } x > 0 \\ 0 & \text{otherwise} \end{cases}$";

#[test]
fn cases_rows_are_not_wrapped_in_literal_brackets() {
    let out = typ(SIMPLE);
    assert!(
        !out.contains("cases(["),
        "a `[` after `cases(` renders as a literal bracket glyph; got:\n{out}"
    );
    assert!(
        !out.contains("],"),
        "no bracketed row separators should survive; got:\n{out}"
    );
}

#[test]
fn cases_uses_the_ampersand_column_separator() {
    // LaTeX's `&` aligns value vs condition into two columns; Typst's `cases()`
    // supports `&` natively, so flattening it to `quad` loses that alignment.
    // Scope the check to the `cases(...)` call itself — asserting on the whole
    // document would pass on any stray `&` elsewhere even if this emitter
    // reverted to `quad` (review finding).
    let out = typ(SIMPLE);
    let inner = cases_inner(&out);
    assert!(
        inner.contains('&'),
        "the `&` column separator should be preserved; got inner:\n{inner}"
    );
    assert!(
        !inner.contains("quad"),
        "`&` must not be flattened back to `quad`; got inner:\n{inner}"
    );
}

#[test]
fn a_top_level_comma_in_a_row_is_protected_not_a_row_separator() {
    // `\max\{a, 0\}` has a comma that must NOT split the row into two cases.
    let out = typ(r"$f = \begin{cases} \max\{a, 0\} & x > 0 \\ 0 & \text{else} \end{cases}$");
    assert!(
        out.contains("\",\""),
        "a top-level comma must be emitted as the quoted string `\",\"`; got:\n{out}"
    );
    assert!(
        !out.contains("cases(["),
        "and still no literal brackets; got:\n{out}"
    );
}

#[test]
fn a_comma_nested_in_a_call_is_left_alone() {
    // `frac(a, b)`'s comma is a real argument separator that Typst's own paren
    // nesting already protects — quoting it would break the call.
    let out = typ(r"$f = \begin{cases} \frac{a}{b} & x > 0 \\ 0 & \text{else} \end{cases}$");
    assert!(
        !out.contains("frac(a\",\"") && !out.contains("frac(a \",\""),
        "a comma inside frac(...) must stay a real separator; got:\n{out}"
    );
}

#[test]
fn two_rows_still_produce_two_cases() {
    // Guard the original reason the brackets existed: row count must not change.
    let out = typ(SIMPLE);
    let inner = cases_inner(&out);
    let top_level_commas = {
        let (mut depth, mut n, mut in_str) = (0i32, 0usize, false);
        let mut prev = '\0';
        for c in inner.chars() {
            match c {
                '"' if prev != '\\' => in_str = !in_str,
                '(' | '[' | '{' if !in_str => depth += 1,
                ')' | ']' | '}' if !in_str => depth -= 1,
                ',' if !in_str && depth == 0 => n += 1,
                _ => {}
            }
            prev = c;
        }
        n
    };
    assert_eq!(
        top_level_commas, 1,
        "two rows ⇒ exactly one separating comma; got inner:\n{inner}"
    );
}

// ─── review findings (all verified against real `typst compile`) ─────────────

#[test]
fn an_unmatched_closer_does_not_reopen_top_level() {
    // Review finding H1. A stray `)` drove the depth counter to -1; a later `(`
    // brought it back to 0, so `binom(n, k)`'s GENUINE argument separator got
    // quoted → `binom(n"," k)` → `error: missing argument: lower`. A row that
    // rendered fine before this PR would have failed the whole document build.
    let out = typ(r"$f=\begin{cases} a) + \binom{n}{k} & x>0 \\ 0 & y \end{cases}$");
    assert!(
        !out.contains("binom(n\",\""),
        "binom's separator must stay a real comma; got:\n{out}"
    );
}

#[test]
fn a_comma_inside_an_escaped_bracket_is_still_protected() {
    // Review finding H2. `escape_unbalanced_math_brackets` turns the unmatched
    // `[`/`)` of a half-open interval into `\[`/`\)`, which do NOT scope in
    // Typst — so a comma the protector had judged "nested" becomes a real
    // separator and `[0,1)` split the row in two (3 rows rendered, not 2).
    // Escaping must therefore happen BEFORE protection, so the protector sees
    // exactly what Typst will see.
    let out = typ(r"$f=\begin{cases} \binom{n}{k} & x \in [0,1) \\ 0 & y \end{cases}$");
    assert!(
        out.contains("\\[0\",\"1\\)"),
        "the interval's comma must be quoted once its brackets are escaped; got:\n{out}"
    );
}

#[test]
fn array_in_math_piecewise_has_no_stray_brackets_either() {
    // Review finding M1. `\left\{\begin{array}{ll}...` is the other very common
    // piecewise idiom and routes through `emit_array_in_math`, which still had
    // the `[...]`-wrapping form this PR removed from `emit_cases_env`.
    let out = typ(r"$f=\left\{\begin{array}{ll} 1 & \text{if } x>0 \\ 0 & \text{else} \end{array}\right.$");
    assert!(
        !out.contains("cases(["),
        "the array path must not emit bracketed rows either; got:\n{out}"
    );
    assert!(out.contains('&'), "and it should keep `&` alignment; got:\n{out}");
}
