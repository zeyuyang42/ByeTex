//! Regression tests for `minipage` environment arguments.
//!
//! `\begin{minipage}[pos][height][inner-pos]{width}` takes a mandatory
//! `{width}` argument (and up to three optional `[...]` positions). minipage is
//! treated as a transparent body wrapper, but its argument groups were emitted
//! as body content: the `{width}` group leaked as a stray `{}` and the
//! `\linewidth`/`\textwidth` inside it produced an `unsupported_command`
//! warning. The arguments must be skipped. See arXiv:2605.22820 (13 such
//! warnings from `\begin{minipage}[t]{\linewidth}`).

use byetex_core::{convert, ConvertOptions};

fn convert_src(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

/// The `{\linewidth}` width arg must not warn as an unsupported command.
#[test]
fn minipage_width_arg_no_warning() {
    let out = convert_src(r"\begin{minipage}[t]{\linewidth} hello \end{minipage}");
    let has_width_warning = out.warnings.iter().any(|w| {
        let c = format!("{:?}", w.category);
        c.contains("\\linewidth") || c.contains("\\textwidth") || c.contains("\\columnwidth")
    });
    assert!(
        !has_width_warning,
        "minipage width arg must not warn;\nwarnings:\n{:#?}",
        out.warnings
    );
}

/// The body content survives, and the width arg does not leak as stray `{}`.
#[test]
fn minipage_body_preserved_no_stray_braces() {
    let out = convert_src(r"\begin{minipage}[t]{\linewidth} hello world \end{minipage}");
    assert!(
        out.typst.contains("hello world"),
        "minipage body must be preserved;\noutput:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("{}"),
        "minipage width arg must not leak as stray `{{}}`;\noutput:\n{}",
        out.typst
    );
}

/// A fractional `{0.5\textwidth}` width with no optional bracket also works.
#[test]
fn minipage_fractional_width_only() {
    let out = convert_src(r"\begin{minipage}{0.5\textwidth} text here \end{minipage}");
    let has_width_warning = out
        .warnings
        .iter()
        .any(|w| format!("{:?}", w.category).contains("textwidth"));
    assert!(!has_width_warning, "fractional width must not warn");
    assert!(
        out.typst.contains("text here"),
        "body must survive;\noutput:\n{}",
        out.typst
    );
}

/// Multiple optional bracket groups before the width (`[pos][height]{width}`)
/// must all be skipped along with the width — none should leak as text/`{}` or
/// warn. tree-sitter folds the first `[..]` into the `begin` node but emits
/// later ones as bare `[`/text/`]` tokens, so the skip must reach the width
/// regardless.
#[test]
fn minipage_multiple_bracket_args() {
    let out = convert_src(r"\begin{minipage}[t][5cm]{\linewidth} body text \end{minipage}");
    let has_width_warning = out
        .warnings
        .iter()
        .any(|w| format!("{:?}", w.category).contains("\\linewidth"));
    assert!(
        !has_width_warning,
        "multi-bracket minipage width arg must not warn;\nwarnings:\n{:#?}",
        out.warnings
    );
    assert!(
        out.typst.contains("body text"),
        "body must survive;\noutput:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("5cm") && !out.typst.contains("{}"),
        "neither the [5cm] height arg nor a stray `{{}}` may leak;\noutput:\n{}",
        out.typst
    );
}

/// Body content that itself starts with a `{...}` group must NOT be eaten — only
/// the single mandatory width group is skipped.
#[test]
fn minipage_only_skips_one_curly() {
    let out = convert_src(r"\begin{minipage}{\linewidth}{\bfseries Bold} rest \end{minipage}");
    assert!(
        out.typst.contains("Bold") && out.typst.contains("rest"),
        "content after width arg (incl. a leading group) must survive;\noutput:\n{}",
        out.typst
    );
}
