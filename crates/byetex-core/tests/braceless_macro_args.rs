//! Tests for brace-less `\newcommand` argument consumption.
//!
//! Before the fix, ByeTex required curly-group arguments for every
//! `\newcommand` call: `\mat{X}` worked, but `\mat X` produced a
//! `custom_macro` warning because the AST has no `curly_group` child.
//! Real arXiv papers (e.g. corpus/online/arxiv/paper) use the brace-less
//! form pervasively — one paper hit 989 `\mat X`-style calls.

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

fn custom_macro_warnings(out: &byetex_core::ConvertOutput) -> Vec<String> {
    out.warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::CustomMacro { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn single_letter_arg() {
    // `\mat X` should expand the same way `\mat{X}` does.
    let src = r"\documentclass{article}
\newcommand{\mat}[1]{\mathbf{#1}}
\begin{document}
$\mat X$
\end{document}";
    let out = convert_str(src);
    assert!(
        custom_macro_warnings(&out).is_empty(),
        "expected no custom_macro warnings for \\mat, got {:?}",
        custom_macro_warnings(&out)
    );
    // The body should contain a bold-math wrapper around X; the exact
    // Typst surface is `bold(X)` in the current emitter.
    assert!(
        out.typst.contains("bold(X)") || out.typst.contains("bold( X )"),
        "expected `bold(X)` in output, got:\n{}",
        out.typst
    );
}

#[test]
fn backslash_command_arg() {
    // `\mat \alpha` — the next token is itself a command.
    let src = r"\documentclass{article}
\newcommand{\mat}[1]{\mathbf{#1}}
\begin{document}
$\mat \alpha$
\end{document}";
    let out = convert_str(src);
    assert!(custom_macro_warnings(&out).is_empty());
    // The alpha must appear in the rendered output (as Typst's `alpha`).
    assert!(
        out.typst.contains("alpha"),
        "alpha missing from output:\n{}",
        out.typst
    );
}

#[test]
fn explicit_curly_still_wins() {
    // When both forms are available — `\mat{XY}` — the curly-group
    // path must take precedence and yield the full inner content.
    let src = r"\documentclass{article}
\newcommand{\mat}[1]{\mathbf{#1}}
\begin{document}
$\mat{XY}$ and $\mat{Z}$
\end{document}";
    let out = convert_str(src);
    assert!(custom_macro_warnings(&out).is_empty());
    // Both pairs should expand to bold-math; the XY pair must keep both
    // chars together (regression guard: brace-less fallback must not
    // override an explicit curly_group).
    assert!(
        out.typst.contains("bold(X Y)") || out.typst.contains("bold(XY)"),
        "expected `bold(XY)` from explicit curly, got:\n{}",
        out.typst
    );
}

#[test]
fn multi_arg_braceless_sequence() {
    // 3-arg macro called brace-less three times in a row.
    // Use parens (not brackets) in the body so we don't fight Typst's
    // text-mode `[...]` content-block escaping in the assertion.
    let src = r"\documentclass{article}
\newcommand{\triple}[3]{(#1|#2|#3)}
\begin{document}
\triple A B C
\end{document}";
    let out = convert_str(src);
    assert!(
        custom_macro_warnings(&out).is_empty(),
        "got warnings: {:?}",
        custom_macro_warnings(&out)
    );
    assert!(
        out.typst.contains("(A|B|C)"),
        "expected `(A|B|C)` in output, got:\n{}",
        out.typst
    );
}

#[test]
fn braceless_with_nonascii() {
    // A non-ASCII codepoint as the arg must not panic the consumer.
    // `é` is 2 bytes in UTF-8; the helper must advance by `len_utf8()`.
    let src = "\\documentclass{article}\n\
        \\newcommand{\\mat}[1]{\\mathbf{#1}}\n\
        \\begin{document}\n\
        $\\mat é$\n\
        \\end{document}";
    let out = convert_str(src);
    // Conversion must not panic; whether `é` round-trips depends on the
    // math emitter's symbol table, but we should not warn about a
    // missing arg.
    assert!(custom_macro_warnings(&out).is_empty());
}

#[test]
fn nested_braceless_macros() {
    // `\matuul H` where `\matuul` is defined in terms of `\mat`, which
    // is defined in terms of `\bm`. Mirrors the real arXiv paper
    // pattern at corpus/online/arxiv/paper/BB-Formats.tex.
    let src = r"\documentclass{article}
\newcommand{\mat}[1]{\mathbf{#1}}
\newcommand{\matuul}[1]{\underline{\underline{\mat{#1}}}}
\begin{document}
$\matuul H$
\end{document}";
    let out = convert_str(src);
    assert!(
        custom_macro_warnings(&out).is_empty(),
        "got warnings: {:?}",
        custom_macro_warnings(&out)
    );
    // Should at least mention H in bold and double-underline form.
    assert!(out.typst.contains("H"), "H missing from output");
}

#[test]
fn brace_group_recursion_capped_by_depth() {
    // A user macro that recursively reaches through a math-wrap
    // brace-group (`\hat{\rec}`). Pre-fix, the Group branch of
    // `emit_math_wrap` created a sub-emitter without bumping
    // `macro_depth`, so the recursion cap was never reached and the
    // stack overflowed. After the fix the cap fires and a
    // CustomMacro warning is emitted.
    let src = r"\documentclass{article}
\newcommand{\rec}{\hat{\rec}}
\begin{document}
$\rec$
\end{document}";
    let out = convert_str(src);
    // The conversion must complete (no stack overflow) and emit a
    // warning identifying the runaway macro.
    let recursion_warn = out.warnings.iter().any(|w| match &w.category {
        Category::CustomMacro { name } => name == "\\rec" && w.message.contains("depth"),
        _ => false,
    });
    assert!(
        recursion_warn,
        "expected a CustomMacro depth-cap warning for \\rec; got: {:?}",
        out.warnings
    );
}

#[test]
fn missing_arg_still_warns() {
    // EOF immediately after a 1-arg macro call: keep the existing
    // "expected 1 arg(s), found 0" warning. Regression guard so the
    // brace-less fallback doesn't accidentally suppress real errors.
    // Source ends with `\mat` followed by only whitespace — the
    // helper returns None and we fall through to the warning branch.
    let src = "\\newcommand{\\mat}[1]{\\mathbf{#1}}\n\\mat   \n";
    let out = convert_str(src);
    assert!(
        !custom_macro_warnings(&out).is_empty(),
        "expected a custom_macro warning when no arg follows the call; got typst:\n{}",
        out.typst
    );
}
