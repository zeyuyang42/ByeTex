//! Tests for end-of-group scope tracking of TeX font-style declarations
//! inside math mode (`\bf`, `\it`, `\rm`, `\sf`, `\tt`, ...).
//!
//! In LaTeX these are *declarations*: they affect every following token
//! up to the end of the enclosing group. ByeTex previously emitted an
//! `ambiguous_math` warning and dropped the command entirely. The
//! single-paper outlier 2605.22281 had 762 `\bf` calls — by far the
//! biggest residual `ambiguous_math` cluster after PRs #22/#23/#27.
//!
//! The fix wraps the post-declaration content in the matching Typst
//! math wrapper (`bold(...)`, `italic(...)`, `upright(...)`, `mono(...)`),
//! confined to the enclosing math container (curly group or `$...$`).
//! Per the standing "partial render only — keep warnings for lossy
//! cases" constraint, this isn't bit-exact (sequential `\bf \it` nests
//! rather than swaps axes) but it preserves visual intent and stops the
//! warning flood.

use byetex_core::{convert, Category, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
}

fn ambiguous_math_messages(out: &byetex_core::ConvertOutput) -> Vec<String> {
    out.warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::AmbiguousMath { reason } => Some(reason.clone()),
            _ => None,
        })
        .collect()
}

fn unsupported_command_names(out: &byetex_core::ConvertOutput) -> Vec<String> {
    out.warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::UnsupportedCommand { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn bf_in_curly_group_wraps_rest() {
    // `{\bf X}` — the `\bf` declaration scopes to the end of the curly
    // group. `X` (and only `X`) should appear inside `bold(...)`.
    let src = r"\documentclass{article}\begin{document}${\bf X}$\end{document}";
    let out = convert_str(src);
    assert!(
        ambiguous_math_messages(&out)
            .iter()
            .all(|m| !m.contains("\\bf")),
        "should not warn about \\bf; got: {:?}",
        ambiguous_math_messages(&out)
    );
    assert!(
        unsupported_command_names(&out)
            .iter()
            .all(|n| n != "\\bf"),
        "should not emit unsupported_command for \\bf; got: {:?}",
        unsupported_command_names(&out)
    );
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("bold(X)"),
        "expected `bold(X)`; got:\n{}",
        out.typst
    );
}

#[test]
fn bf_in_inline_math_wraps_rest_of_span() {
    // `$\bf x$` — no curly group, the entire `$...$` is the scope.
    let src = r"\documentclass{article}\begin{document}$\bf x$\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("bold(x)"),
        "expected `bold(x)`; got:\n{}",
        out.typst
    );
}

#[test]
fn it_in_curly_group_wraps_rest() {
    let src = r"\documentclass{article}\begin{document}${\it x}$\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("italic(x)"),
        "expected `italic(x)`; got:\n{}",
        out.typst
    );
}

#[test]
fn rm_in_curly_group_wraps_upright() {
    let src = r"\documentclass{article}\begin{document}${\rm x}$\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("upright(x)"),
        "expected `upright(x)`; got:\n{}",
        out.typst
    );
}

#[test]
fn tt_in_curly_group_wraps_mono() {
    let src = r"\documentclass{article}\begin{document}${\tt x}$\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("mono(x)"),
        "expected `mono(x)`; got:\n{}",
        out.typst
    );
}

#[test]
fn bf_with_preceding_content() {
    // `${a \bf b}$` — `a` is normal weight; `b` is bold.
    let src = r"\documentclass{article}\begin{document}${a \bf b}$\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("abold(b)"),
        "expected `a bold(b)` (with the leading `a` outside the wrapper); got:\n{}",
        out.typst
    );
}

#[test]
fn bf_does_not_leak_outside_group() {
    // `${\bf x} y$` — only `x` is bold; `y` is plain.
    let src = r"\documentclass{article}\begin{document}${\bf x} y$\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    // `bold(x)` must appear, and `y` must appear *outside* any `bold(`
    // wrapper. The simplest way to assert that is that `y` shows up
    // after the closing `)` of `bold(x)`.
    let pos_bold_close = stripped.find("bold(x)").map(|i| i + "bold(x)".len());
    assert!(pos_bold_close.is_some(), "no bold(x) in:\n{}", out.typst);
    let tail = &stripped[pos_bold_close.unwrap()..];
    assert!(
        tail.contains('y'),
        "expected y after bold(x); got tail `{}` in:\n{}",
        tail,
        out.typst
    );
}

#[test]
fn nested_font_decls_nest_in_typst() {
    // `${\bf a \it b}$` — partial-render: nest italic inside bold.
    // (Strictly, LaTeX would set b to bold-italic, not nested. We do
    // the partial-fidelity render that keeps both intents visible.)
    let src = r"\documentclass{article}\begin{document}${\bf a \it b}$\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("bold(aitalic(b))"),
        "expected `bold(a italic(b))`; got:\n{}",
        out.typst
    );
}

#[test]
fn bfseries_alias_works() {
    // `\bfseries` is the LaTeX2e form of `\bf`. Same scope semantics.
    let src = r"\documentclass{article}\begin{document}${\bfseries x}$\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("bold(x)"),
        "expected `bold(x)` from \\bfseries; got:\n{}",
        out.typst
    );
}

#[test]
fn empty_after_bf_emits_empty_wrapper() {
    // `${\bf}$` — declaration with no scope content. Emit `bold()`
    // (or any empty wrapper); no panic, no warning.
    let src = r"\documentclass{article}\begin{document}${\bf}$\end{document}";
    let out = convert_str(src);
    // Just make sure the conversion doesn't panic and doesn't emit an
    // unsupported_command warning. Output shape is incidental.
    assert!(
        unsupported_command_names(&out)
            .iter()
            .all(|n| n != "\\bf"),
        "no unsupported_command for \\bf; got: {:?}",
        unsupported_command_names(&out)
    );
}
