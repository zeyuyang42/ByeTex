//! Regression tests for inline code commands `\lstinline` (listings) and
//! `\mintinline` (minted).
//!
//! Both typeset their argument verbatim, like `\verb`, but with extra syntax:
//!   `\lstinline[opts]|code|` / `\lstinline[opts]{code}`
//!   `\mintinline{lang}|code|` / `\mintinline{lang}{code}`
//! They were unhandled (warned + leaked content). They must become inline
//! `#raw("...")` (with `lang:` when a language is known). See arXiv:2605.22800.

use byetex_core::{convert, ConvertOptions};

fn convert_src(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

fn no_unsupported(out: &byetex_core::ConvertOutput, name: &str) -> bool {
    !out.warnings
        .iter()
        .any(|w| format!("{:?}", w.category).contains(name))
}

/// `\lstinline|code|` (pipe delimiter) → `#raw("code")`, underscore preserved.
#[test]
fn lstinline_pipe_delimiter() {
    let out = convert_src(r"\lstinline|x_int = 5|");
    assert!(
        out.typst.contains(r#"#raw("x_int = 5")"#),
        "expected #raw(\"x_int = 5\");\noutput:\n{}",
        out.typst
    );
    assert!(no_unsupported(&out, "lstinline"), "must not warn");
}

/// `\lstinline{code}` (brace form) → `#raw("code")`.
#[test]
fn lstinline_brace_form() {
    let out = convert_src(r"\lstinline{y = f(x)}");
    assert!(
        out.typst.contains(r#"#raw("y = f(x)")"#),
        "expected #raw(\"y = f(x)\");\noutput:\n{}",
        out.typst
    );
    assert!(no_unsupported(&out, "lstinline"), "must not warn");
}

/// `\lstinline[language=Python]|code|` → `#raw("code", lang: "python")`.
#[test]
fn lstinline_with_language_option() {
    let out = convert_src(r"\lstinline[language=Python]|a_b|");
    assert!(
        out.typst.contains(r#"#raw("a_b", lang: "python")"#),
        "expected #raw with lang python;\noutput:\n{}",
        out.typst
    );
    assert!(no_unsupported(&out, "lstinline"), "must not warn");
}

/// An alternate delimiter (`!`) works, same as `\verb`.
#[test]
fn lstinline_alternate_delimiter() {
    let out = convert_src(r"\lstinline!foo_bar!");
    assert!(
        out.typst.contains(r#"#raw("foo_bar")"#),
        "expected #raw(\"foo_bar\");\noutput:\n{}",
        out.typst
    );
}

/// `\mintinline{python}|code|` → `#raw("code", lang: "python")`
/// (minted's first mandatory arg is the language).
#[test]
fn mintinline_lang_then_pipe() {
    let out = convert_src(r"\mintinline{python}|z_1 = 2|");
    assert!(
        out.typst.contains(r#"#raw("z_1 = 2", lang: "python")"#),
        "expected #raw with lang python;\noutput:\n{}",
        out.typst
    );
    assert!(no_unsupported(&out, "mintinline"), "must not warn");
}

/// `\mintinline{c}{code}` (brace code) → `#raw("code", lang: "c")`.
#[test]
fn mintinline_lang_then_brace() {
    let out = convert_src(r"\mintinline{c}{int x = 0;}");
    assert!(
        out.typst.contains(r#"#raw("int x = 0;", lang: "c")"#),
        "expected #raw with lang c;\noutput:\n{}",
        out.typst
    );
}

/// `\lstinline` content is verbatim: a `"` and `\` must be escaped for the
/// Typst string but otherwise preserved.
#[test]
fn lstinline_escapes_quote_and_backslash() {
    let out = convert_src(r#"\lstinline|a"b\c|"#);
    assert!(
        out.typst.contains(r#"#raw("a\"b\\c")"#),
        "quote/backslash must be escaped for the string literal;\noutput:\n{}",
        out.typst
    );
}

/// Text after the command must be preserved exactly once (the handler must not
/// over- or under-consume), for both the brace and the delimiter form.
#[test]
fn lstinline_trailing_text_preserved() {
    let out = convert_src(r"start \lstinline{x} mid \lstinline|y| end");
    assert!(
        out.typst.contains(r#"#raw("x")"#) && out.typst.contains(r#"#raw("y")"#),
        "both inline codes emitted;\noutput:\n{}",
        out.typst
    );
    // surrounding words survive, each exactly once (no over/under-consume)
    for word in ["start", "mid", "end"] {
        assert_eq!(
            out.typst.matches(word).count(),
            1,
            "`{word}` must appear exactly once;\noutput:\n{}",
            out.typst
        );
    }
}

/// Brace-form code may itself contain balanced braces.
#[test]
fn lstinline_brace_code_with_inner_braces() {
    let out = convert_src(r"\lstinline{f({x})}");
    assert!(
        out.typst.contains(r#"#raw("f({x})")"#),
        "balanced inner braces must be captured;\noutput:\n{}",
        out.typst
    );
}

/// `\lstinline` inside a table cell keeps its underscore (the cell-escaping pass
/// copies `#raw(...)` verbatim).
#[test]
fn lstinline_in_table_cell_keeps_underscore() {
    let out = convert_src("\\begin{tabular}{ll}\n\\lstinline|a_b| & x \\\\\n\\end{tabular}");
    assert!(
        out.typst.contains(r#"#raw("a_b")"#),
        "table-cell lstinline underscore must survive;\noutput:\n{}",
        out.typst
    );
}

/// Unterminated forms must degrade gracefully — no panic / hang. (We only assert
/// the conversion returns; content handling for malformed input is best-effort.)
#[test]
fn lstinline_unterminated_does_not_panic() {
    let _ = convert_src(r"\lstinline|foo");
    let _ = convert_src(r"\lstinline{foo");
    let _ = convert_src(r"\mintinline{py}");
    let _ = convert_src(r"\lstinline");
}
