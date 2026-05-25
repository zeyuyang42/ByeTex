//! Tests for the brace-less `\newcommand\name{body}` definition form.
//!
//! tree-sitter-latex parses `\newcommand\name{body}` with a direct
//! `command_name` child as the `declaration` field — not the
//! `curly_group_command_name` child the canonical `\newcommand{\name}{body}`
//! form produces. ByeTex's `extract_newcommand` previously only handled
//! the canonical form, so any arXiv paper defining macros in the
//! brace-less form (very common in `style/header.tex` files) saw every
//! call site emit `ambiguous_math`.

use std::fs;
use std::path::{Path, PathBuf};

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

#[test]
fn braceless_def_name_no_args() {
    // `\newcommand\foo{X}` — the brace-less name form, zero parameters.
    let src = r"\documentclass{article}
\newcommand\foo{X}
\begin{document}
$\foo$
\end{document}";
    let out = convert_str(src);
    let amb: Vec<_> = ambiguous_math_messages(&out)
        .into_iter()
        .filter(|m| m.contains("foo"))
        .collect();
    assert!(
        amb.is_empty(),
        "expected no ambiguous_math warnings for \\foo; got: {:?}",
        amb
    );
    // Expansion produces an X.
    assert!(
        out.typst.contains("X"),
        "expected X in output, got:\n{}",
        out.typst
    );
}

#[test]
fn braceless_def_name_with_arity() {
    // `\newcommand\bar[1]{(#1)}` — brace-less name + parameter count.
    // Combined with PR #9's brace-less call support, `\bar y` should
    // expand to `(y)`.
    let src = r"\documentclass{article}
\newcommand\bar[1]{(#1)}
\begin{document}
$\bar y$
\end{document}";
    let out = convert_str(src);
    let amb: Vec<_> = ambiguous_math_messages(&out)
        .into_iter()
        .filter(|m| m.contains("bar"))
        .collect();
    assert!(
        amb.is_empty(),
        "expected no ambiguous_math warnings for \\bar; got: {:?}",
        amb
    );
    // y wrapped in parens — exact rendering may vary slightly but the
    // brace and the argument letter should both appear.
    assert!(
        out.typst.contains("(y)") || out.typst.contains("( y )"),
        "expected `(y)` from \\bar y; got:\n{}",
        out.typst
    );
}

#[test]
fn canonical_form_still_works() {
    // Regression guard: the canonical `\newcommand{\baz}{Z}` form must
    // still extract correctly. The new brace-less branch should not
    // interfere with the existing curly path.
    let src = r"\documentclass{article}
\newcommand{\baz}{Z}
\begin{document}
$\baz$
\end{document}";
    let out = convert_str(src);
    let amb: Vec<_> = ambiguous_math_messages(&out)
        .into_iter()
        .filter(|m| m.contains("baz"))
        .collect();
    assert!(
        amb.is_empty(),
        "expected no ambiguous_math warnings for canonical \\baz; got: {:?}",
        amb
    );
    assert!(out.typst.contains("Z"));
}

#[test]
fn braceless_def_with_optional_default_expands_via_default() {
    // `\newcommand\foo[1][default]{body}` — optional-default form.
    // Previously bailed; now harvested via the optional_defaults map
    // on MacroDef. Calling `\foo` (no bracket) substitutes `default`
    // for `#1`. See newcommand_optional_default.rs for full coverage.
    let src = r"\documentclass{article}
\newcommand\foo[1][default]{[[#1]]}
\begin{document}
$\foo$
\end{document}";
    let out = convert_str(src);
    let amb: Vec<_> = ambiguous_math_messages(&out)
        .into_iter()
        .filter(|m| m.contains("foo"))
        .collect();
    assert!(
        amb.is_empty(),
        "no ambiguous_math expected for \\foo (now harvested); got: {:?}",
        amb
    );
    // `default` should appear in the output (substituted into `#1`).
    // Typst math splits multi-letter words char-by-char, so the value
    // may show up as `d e f a u l t` rather than `default`.
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("default"),
        "expected `default` (substituted value) in output; got:\n{}",
        out.typst
    );
}

#[test]
fn input_chain_inherits_pre_scanned_macros() {
    // Real-world arXiv pattern: macros defined in `style/header.tex`,
    // entry `\input`s the style file AND chapter files, the chapter
    // files use the macros in math.
    //
    // Pre-`expand_latex_include` inheritance fix: sub-emitter for each
    // `\input` started with empty macros. So every `$\src$` call in
    // `chapter.tex` emitted `ambiguous_math` even though the pre-scan
    // had successfully harvested `\src` into the parent's table.
    //
    // Post-fix: sub-emitters inherit the parent's macro table, so
    // the macros propagate across the `\input` chain.
    let dir = tmpdir("input-chain");
    write(
        &dir,
        "style/header.tex",
        "\\newcommand\\src{\\nu_{\\text{src}}}\n\
         \\newcommand\\norm[1]{\\left\\|#1\\right\\|}\n",
    );
    write(
        &dir,
        "chapter.tex",
        "The source is $\\src$, with norm $\\norm{x}$.\n",
    );
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\
         \\input{style/header.tex}\n\
         \\begin{document}\n\
         \\input{chapter.tex}\n\
         \\end{document}\n",
    );
    let plan = byetex_core::project::plan_project_from_dir(&dir, true).unwrap();
    let unrecognised: Vec<_> = plan
        .warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::AmbiguousMath { reason } => Some(reason.clone()),
            _ => None,
        })
        .filter(|m| m == "\\src" || m == "\\norm")
        .collect();
    assert!(
        unrecognised.is_empty(),
        "macros pre-scanned from style/header.tex must reach the \
         sub-emitter for chapter.tex; got: {:?}\ntypst: {}",
        unrecognised,
        plan.main_typst
    );
}

#[test]
#[ignore = "diagnostic probe — run with --ignored when debugging"]
fn probe_real_header_tex() {
    // Real-world snippet from corpus/online/arxiv/2605.22507/source/style/header.tex.
    // Drives `plan_project_from_dir` against an in-memory copy to check
    // whether brace-less defs survive the round trip.
    let dir = tmpdir("probe-real");
    write(
        &dir,
        "style/header.tex",
        "\\newcommand\\pSrc{p_{\\text{src}}}\n\
         \\newcommand\\Pm{\\mathbb{P}}\n\
         \\newcommand\\supp{\\text{supp}}\n\
         \\newcommand\\opt{\\text{OPT}}\n\
         \n\
         \\newcommand\\src{\\nu_{\\text{src}}}\n\
         \\newcommand\\tgt{\\nu_{\\text{tgt}}}\n\
         \\newcommand\\norm[1]{\\left\\|#1\\right\\|}\n",
    );
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\
         \\input{style/header}\n\
         \\begin{document}\n\
         The distance $\\norm{\\src - \\tgt}$ and $\\supp(X)$.\n\
         \\end{document}\n",
    );
    let plan = byetex_core::project::plan_project_from_dir(&dir, true).unwrap();
    let unrecognised: Vec<String> = plan
        .warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::AmbiguousMath { reason } => Some(reason.clone()),
            _ => None,
        })
        .filter(|m| {
            m.contains("src") || m.contains("tgt") || m.contains("norm") || m.contains("supp")
        })
        .collect();
    eprintln!(
        "ambiguous_math for fixture (should be empty): {:?}",
        unrecognised
    );
    eprintln!("typst output:\n{}", plan.main_typst);
    assert!(
        unrecognised.is_empty(),
        "header.tex defs not harvested: {:?}",
        unrecognised
    );
}

#[test]
fn braceless_def_in_sibling_file_is_pre_scanned() {
    // arXiv pattern: macros live in `style/header.tex`, the entry
    // doesn't `\input` them but the folder-mode pre-scan must still
    // pick them up. Uses the brace-less name form throughout because
    // that's what triggers the bug.
    let dir = tmpdir("braceless-def-sibling");
    write(
        &dir,
        "style/header.tex",
        "\\newcommand\\src{\\nu_{\\text{src}}}\n\
         \\newcommand\\tgt{\\nu_{\\text{tgt}}}\n\
         \\newcommand\\norm[1]{\\left\\|#1\\right\\|}\n",
    );
    write(
        &dir,
        "main.tex",
        "\\documentclass{article}\n\
         \\input{style/header}\n\
         \\begin{document}\n\
         The distance $\\norm{\\src - \\tgt}$ is small.\n\
         \\end{document}\n",
    );

    // Plan via the folder pipeline to exercise the pre-scan.
    let plan = byetex_core::project::plan_project_from_dir(&dir, true).unwrap();
    let amb_msgs: Vec<_> = plan
        .warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::AmbiguousMath { reason } => Some(reason.clone()),
            _ => None,
        })
        .collect();
    let unrecognised: Vec<_> = amb_msgs
        .iter()
        .filter(|m| m.contains("src") || m.contains("tgt") || m.contains("norm"))
        .collect();
    assert!(
        unrecognised.is_empty(),
        "expected pre-scan to harvest brace-less defs from style/header.tex; \
         got ambiguous_math: {:?}\nfull typst:\n{}",
        unrecognised,
        plan.main_typst
    );
}

// --------------------------- helpers ---------------------------

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "byetex-braceless-def-{}-{}",
        name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn wrapper_newcommand_is_harvested() {
    // Macros defined indirectly via a wrapper like:
    //   \newcommand{\mytoken}[2]{\newcommand{#1}{body}}
    //   \mytoken{\token}{t}
    // should result in \token being available at call sites.
    let src = concat!(
        r"\newcommand{\mytoken}[2]{\newcommand{#1}{{#2}}}",
        "\n",
        r"\mytoken{\token}{t}",
        "\n",
        r"\mytoken{\vocab}{\mathcal{T}}",
        "\n",
        r"\begin{document}",
        "\n",
        "$\\token$ and $\\vocab$",
        "\n",
        r"\end{document}",
    );
    let out = byetex_core::convert(src, &byetex_core::ConvertOptions::default());
    let ambiguous: Vec<_> = out
        .warnings
        .iter()
        .filter(|w| format!("{:?}", w.category).contains("ambiguous_math"))
        .collect();
    assert!(
        ambiguous.is_empty(),
        "expected \\token and \\vocab to expand via wrapper harvest; got: {ambiguous:?}"
    );
}

#[test]
fn wrapper_newcommand_with_color_in_body() {
    // Exact 22821 pattern: body has \color{...} which contains nested commands
    let src = concat!(
        "\\newcommand{\\mytoken}[2]{\\newcommand{#1}{{\\color{x}#2}}}\n",
        "\\mytoken{\\token}{t}\n",
        "\\mytoken{\\vocab}{\\mathcal{T}}\n",
        "\\begin{document}\n",
        "$\\token$ and $\\vocab$\n",
        "\\end{document}\n",
    );
    let out = byetex_core::convert(src, &byetex_core::ConvertOptions::default());
    let ambiguous: Vec<_> = out
        .warnings
        .iter()
        .filter(|w| format!("{:?}", w.category).contains("ambiguous_math"))
        .collect();
    println!(
        "warnings: {:?}",
        out.warnings
            .iter()
            .map(|w| (format!("{:?}", w.category), &w.snippet))
            .collect::<Vec<_>>()
    );
    assert!(
        ambiguous.is_empty(),
        "expected no ambiguous_math for \\token and \\vocab; got: {ambiguous:?}"
    );
}
