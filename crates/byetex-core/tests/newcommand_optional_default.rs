//! Tests for `\newcommand` with an optional default argument.
//!
//! LaTeX syntax: `\newcommand\foo[N][default]{body}` means the macro
//! takes N total parameters, the first is optional with the given
//! default. ByeTex used to bail on any `brack_group` after
//! `brack_group_argc`, dropping the entire definition silently. Real
//! arXiv papers rely on this form pervasively — 2605.22159's
//! `newcommands.tex` defines `\traceD`, `\genvarBdh`, `\genvarVh`,
//! `\jumpN`, `\traceN` etc. all via `\newcommand[N][]{body}` (empty
//! default). That single paper had ~143 `ambiguous_math` warnings
//! all traceable to this hole.

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

fn ambiguous_messages(out: &byetex_core::ConvertOutput) -> Vec<String> {
    out.warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::AmbiguousMath { reason } => Some(reason.clone()),
            _ => None,
        })
        .collect()
}

fn custom_messages(out: &byetex_core::ConvertOutput) -> Vec<String> {
    out.warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::CustomMacro { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn empty_default_with_call_omitting_optional() {
    // `\newcommand{\foo}[1][]{body[#1]}` — empty default. Calling
    // `\foo` with no `[arg]` substitutes empty string for `#1`.
    let src = r"\documentclass{article}
\newcommand{\foo}[1][]{X#1Y}
\begin{document}
$\foo$
\end{document}";
    let out = convert_str(src);
    assert!(
        ambiguous_messages(&out)
            .iter()
            .chain(custom_messages(&out).iter())
            .all(|m| !m.contains("foo")),
        "no foo warning expected; got ambiguous={:?} custom={:?}",
        ambiguous_messages(&out),
        custom_messages(&out)
    );
    // Output should contain `XY` (the body with `#1` → empty).
    assert!(
        out.typst.contains("X Y") || out.typst.contains("XY"),
        "expected `XY` from body; got:\n{}",
        out.typst
    );
}

#[test]
fn nonempty_default_substituted_when_optional_omitted() {
    // `\newcommand{\trace}[1][\Omega]{T_{#1}}` — default `\Omega`.
    // Calling `\trace` substitutes `\Omega` into `#1`.
    let src = r"\documentclass{article}
\newcommand{\trace}[1][\Omega]{T_{#1}}
\begin{document}
$\trace$
\end{document}";
    let out = convert_str(src);
    assert!(
        ambiguous_messages(&out)
            .iter()
            .chain(custom_messages(&out).iter())
            .all(|m| !m.contains("trace")),
        "no trace warning; got ambiguous={:?} custom={:?}",
        ambiguous_messages(&out),
        custom_messages(&out)
    );
    // The default `\Omega` should expand to `Omega` in the subscript.
    assert!(
        out.typst.contains("Omega") || out.typst.contains("Ω"),
        "expected Omega in output; got:\n{}",
        out.typst
    );
}

#[test]
fn optional_arg_at_call_site_overrides_default() {
    // `\foo[a]` overrides the default; `#1` becomes `a`.
    let src = r"\documentclass{article}
\newcommand{\foo}[1][def]{X#1Y}
\begin{document}
$\foo[a]$
\end{document}";
    let out = convert_str(src);
    assert!(
        ambiguous_messages(&out)
            .iter()
            .chain(custom_messages(&out).iter())
            .all(|m| !m.contains("foo")),
        "no foo warning; got ambiguous={:?} custom={:?}",
        ambiguous_messages(&out),
        custom_messages(&out)
    );
    assert!(
        out.typst.contains("X a Y") || out.typst.contains("XaY"),
        "expected `XaY`; got:\n{}",
        out.typst
    );
}

#[test]
fn two_arg_macro_with_optional_first() {
    // `\newcommand{\customxy}[2][!]{#1#2}` — 2 args, position 1
    // optional. Call `\customxy{y}` → `#1=!, #2=y`. (Don't use `\bar`
    // — that's a reserved math accent.)
    let src = r"\documentclass{article}
\newcommand{\customxy}[2][!]{Z#1W#2}
\begin{document}
$\customxy{y}$
\end{document}";
    let out = convert_str(src);
    assert!(
        ambiguous_messages(&out)
            .iter()
            .chain(custom_messages(&out).iter())
            .all(|m| !m.contains("customxy")),
        "no customxy warning; got ambiguous={:?} custom={:?}",
        ambiguous_messages(&out),
        custom_messages(&out)
    );
    // Output should have `!` followed by `y` somewhere.
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("Z!Wy"),
        "expected `Z!Wy` from default + mandatory; got:\n{}",
        out.typst
    );
}

#[test]
fn real_world_trace_d_pattern() {
    // The exact form from 2605.22159: `\newcommand{\trace}[1][]{...}`
    // with `\trace[\Gamma]` and bare `\trace` both as call sites.
    let src = r"\documentclass{article}
\DeclareMathOperator{\trace}{\gamma}
\newcommand{\traceD}[1][]{\trace_{#1}}
\begin{document}
$\traceD[\Gamma] v = \traceD v$
\end{document}";
    let out = convert_str(src);
    let amb: Vec<_> = ambiguous_messages(&out)
        .into_iter()
        .filter(|m| m.contains("traceD"))
        .collect();
    let cust: Vec<_> = custom_messages(&out)
        .into_iter()
        .filter(|m| m.contains("traceD"))
        .collect();
    assert!(
        amb.is_empty() && cust.is_empty(),
        "no traceD warnings expected; got ambiguous={:?} custom={:?}",
        amb,
        cust
    );
    // Both `\traceD[\Gamma]` and bare `\traceD` should expand.
    assert!(
        out.typst.contains("Gamma") || out.typst.contains("Γ"),
        "expected Gamma in output from \\traceD[\\Gamma]; got:\n{}",
        out.typst
    );
}

#[test]
fn canonical_form_still_works() {
    // Regression: a vanilla `\newcommand{\baz}{Z}` must still work.
    let src = r"\documentclass{article}
\newcommand{\baz}{Z}
\begin{document}
$\baz$
\end{document}";
    let out = convert_str(src);
    let amb: Vec<_> = ambiguous_messages(&out)
        .into_iter()
        .filter(|m| m.contains("baz"))
        .collect();
    assert!(amb.is_empty(), "no baz warning; got: {:?}", amb);
    assert!(out.typst.contains("Z"));
}
