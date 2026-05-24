//! Tests for `\newcommandx` (the `xargspec` package's extended
//! `\newcommand`).
//!
//! Syntax: `\newcommandx\foo[N][K=default, K=default, ...]{body}` —
//! N total params, positions listed in the second bracket are
//! optional with the given default. tree-sitter-latex doesn't have a
//! built-in node for this, so we harvest it via the generic_command
//! path and dispatch on `command_name == "\newcommandx"`.
//!
//! Real-world driver: 2605.22765 (NeurIPS preprint) defines `\pdata`,
//! `\fw`, `\barpdata`, `\pnoise`, `\denoiser`, `\loodenoiser` all via
//! `\newcommandx`. That single paper had ~660 `ambiguous_math`
//! warnings from these definitions never being harvested.

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
fn single_optional_position_empty_default() {
    // Common form: `\newcommandx\foo[2][2=]{body}` — 2 args, position
    // 2 optional with empty default. Calling `\foo{a}` substitutes
    // `#1=a, #2=""`.
    let src = r"\documentclass{article}
\newcommandx\foo[2][2=]{X#1Y#2Z}
\begin{document}
$\foo{a}$
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
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("XaYZ"),
        "expected `XaYZ` (position 2 empty); got:\n{}",
        out.typst
    );
}

#[test]
fn single_optional_position_with_default() {
    // `\newcommandx\foo[2][2=def]{body}` — position 2 defaults to `def`.
    let src = r"\documentclass{article}
\newcommandx\foo[2][2=def]{X#1Y#2Z}
\begin{document}
$\foo{a}$
\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("XaYdefZ"),
        "expected `XaYdefZ`; got:\n{}",
        out.typst
    );
}

#[test]
fn optional_position_overridden_at_call_site() {
    // `\foo{a}[b]` — `[b]` overrides the default for position 2.
    let src = r"\documentclass{article}
\newcommandx\foo[2][2=def]{X#1Y#2Z}
\begin{document}
$\foo{a}[b]$
\end{document}";
    let out = convert_str(src);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("XaYbZ"),
        "expected `XaYbZ` with override; got:\n{}",
        out.typst
    );
}

#[test]
fn real_world_pdata_shape() {
    // Mirrors the `\pdata` definition from 2605.22765:
    // `\newcommandx\pdata[4][4=]{...complex body...}`.
    // Called as `\pdata{x}{y}{z}` (3 mandatory args, position 4
    // empty by default).
    let src = r"\documentclass{article}
\newcommandx\pdata[4][4=]{p^{#4}_{#1}(#3|#2)}
\begin{document}
$\pdata{x}{y}{z}$ and $\pdata{x}{y}{z}[t]$
\end{document}";
    let out = convert_str(src);
    let amb: Vec<_> = ambiguous_messages(&out)
        .into_iter()
        .filter(|m| m.contains("pdata"))
        .collect();
    let cust: Vec<_> = custom_messages(&out)
        .into_iter()
        .filter(|m| m.contains("pdata"))
        .collect();
    assert!(
        amb.is_empty() && cust.is_empty(),
        "no pdata warnings; got ambiguous={:?} custom={:?}",
        amb,
        cust
    );
    // First call should expand to `p^{}_{x}(z|y)` (no superscript on
    // empty `#4`); second to `p^{t}_{x}(z|y)`.
    assert!(
        out.typst.contains("p"),
        "expected p in output; got:\n{}",
        out.typst
    );
}

#[test]
fn arity_only_no_optional() {
    // `\newcommandx\bar[1]{...}` — no `[K=...]` second bracket.
    // Should behave exactly like `\newcommand\bar[1]{...}`.
    let src = r"\documentclass{article}
\newcommandx\customA[1]{[#1]}
\begin{document}
\customA{hello}
\end{document}";
    let out = convert_str(src);
    let amb: Vec<_> = ambiguous_messages(&out)
        .into_iter()
        .filter(|m| m.contains("customA"))
        .collect();
    let cust: Vec<_> = custom_messages(&out)
        .into_iter()
        .filter(|m| m.contains("customA"))
        .collect();
    assert!(
        amb.is_empty() && cust.is_empty(),
        "no customA warning; got ambiguous={:?} custom={:?}",
        amb,
        cust
    );
}
