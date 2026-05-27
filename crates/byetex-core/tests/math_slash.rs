/// Regression tests for leading `/` in math mode (Bug: "unexpected slash" in Typst).
/// In LaTeX, `/` in math mode is a plain slash glyph.
/// In Typst math, `/` is a binary operator — `$/$` and `$/x$` are syntax errors.
/// The fix wraps a leading `/` in a string literal: `$/` → `$"/"`.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn standalone_math_slash_does_not_produce_bare_dollar_slash() {
    // LaTeX: train$/$mini
    let src = r"train$/$mini";
    let out = convert(src);
    assert!(
        !out.typst.contains("$/$"),
        "bare $/ must not appear in output (would cause Typst 'unexpected slash'), got: {}",
        out.typst
    );
}

#[test]
fn standalone_math_slash_emits_quoted_slash() {
    let src = r"train$/$mini";
    let out = convert(src);
    // Should produce $"/"$ or similar quoting of the slash
    assert!(
        out.typst.contains("\"/\""),
        "slash must be quoted as \"/\" in Typst math, got: {}",
        out.typst
    );
}

#[test]
fn leading_slash_in_math_expr() {
    // LaTeX: $/\mathrm{TDI}$
    let src = r"$/\mathrm{TDI}$";
    let out = convert(src);
    assert!(
        !out.typst.starts_with("$/"),
        "leading $/ must not appear in output, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("\"/\""),
        "leading slash must be quoted, got: {}",
        out.typst
    );
}

#[test]
fn fraction_slash_unaffected() {
    // $a/b$ — normal fraction slash, must NOT be changed
    let src = r"$a/b$";
    let out = convert(src);
    // The content should still have a / between a and b
    assert!(
        out.typst.contains("a/b") || out.typst.contains("a / b"),
        "normal fraction a/b must be preserved, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("a\"/\"b"),
        "non-leading slash must NOT be quoted, got: {}",
        out.typst
    );
}
