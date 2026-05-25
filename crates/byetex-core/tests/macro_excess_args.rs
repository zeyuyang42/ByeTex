//! Macros whose body ends in a dangling command, called with more
//! curly-group args than declared.
//!
//! Real arXiv pattern: `\newcommand{\conj}{\overline}` makes `\conj`
//! a zero-arg alias for `\overline`. Calling it as `$\conj{z}$` flows
//! the `{z}` into `\overline`'s arg position in LaTeX's
//! token-streaming expansion model.
//!
//! ByeTex substitutes the body in isolation, so the substituted body
//! `\overline` would warn "missing argument" if we left the caller's
//! `{z}` behind. This test pins the behavior that splices excess
//! curly-group args onto the substituted body before re-parsing.

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

#[test]
fn zero_arg_macro_with_call_site_curly_renders_dangling_command() {
    // `\newcommand{\conj}{\overline}` + `$\conj{z}$` must produce
    // `overline(z)` in Typst.
    let src = r"\documentclass{article}
\newcommand{\conj}{\overline}
\begin{document}
$\conj{z}$
\end{document}";
    let out = convert_str(src);
    let missing_count = out
        .warnings
        .iter()
        .filter(|w| {
            matches!(
                &w.category,
                Category::AmbiguousMath { reason } if reason == "missing argument"
            )
        })
        .count();
    assert_eq!(
        missing_count, 0,
        "expected no `missing argument` warnings for \\conj{{z}}; got typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("overline(z)") || out.typst.contains("overline( z )"),
        "expected `overline(z)`; got:\n{}",
        out.typst
    );
}

#[test]
fn two_excess_args_both_spliced() {
    // `\newcommand{\pair}{}` is a 0-arg macro with empty body.
    // `\pair{a}{b}` flows both `{a}` and `{b}` into the (empty) body.
    // Since the body is empty, nothing renders for them, but no
    // warning should fire — both args were "consumed" by the splice.
    let src = r"\documentclass{article}
\newcommand{\noop}{}
\begin{document}
\noop{a}{b}
\end{document}";
    let out = convert_str(src);
    let missing = out
        .warnings
        .iter()
        .filter(|w| {
            matches!(
                &w.category,
                Category::AmbiguousMath { reason } if reason == "missing argument"
            )
        })
        .count();
    assert_eq!(missing, 0);
}

#[test]
fn declared_arity_matches_call_no_splice() {
    // Regression guard: when the macro takes the args it declares,
    // splicing doesn't add anything. Pure expansion semantics.
    let src = r"\documentclass{article}
\newcommand{\wrap}[1]{[[#1]]}
\begin{document}
$\wrap{x}$
\end{document}";
    let out = convert_str(src);
    // Sanity: \wrap{x} renders with x in the body somewhere.
    assert!(out.typst.contains("x"));
}
