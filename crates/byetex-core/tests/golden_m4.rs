//! M4 golden tests: tables, figures, citations, refs, bibliography.

use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/byetex-core has at least two parents")
        .join("tests/fixtures")
}

fn run(rel: &str) -> String {
    let path = fixtures_root().join(rel);
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let opts = ConvertOptions {
        source_name: Some(rel.to_string()),
        ..Default::default()
    };
    let out = convert(&source, &opts);
    let warnings_json = serde_json::to_string_pretty(&out.warnings).expect("warnings serialize");
    format!(
        "==== TYPST ====\n{}==== WARNINGS ====\n{}\n",
        out.typst, warnings_json
    )
}

#[test]
fn m4_tabular_basic() {
    insta::assert_snapshot!(run("m4_floats/tabular_basic.tex"), @r"
    ==== TYPST ====
    A small table:

    #table(
      columns: 3,
      align: (left, center, right),
      [Name], [Age], [Score],
      [Alice], [30], [95],
      [Bob], [25], [87],
    )
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m4_figure_basic() {
    insta::assert_snapshot!(run("m4_floats/figure_basic.tex"), @r#"
    ==== TYPST ====
    #figure(
      image("example.png", width: 50%),
      caption: [An example figure.],
    ) <fig:ex>

    See Figure @fig:ex for details.
    ==== WARNINGS ====
    []
    "#);
}

#[test]
fn m4_cite_ref() {
    insta::assert_snapshot!(run("m4_floats/cite_ref.tex"), @r#"
    ==== TYPST ====
    #set heading(numbering: "1.")
    #set math.equation(numbering: "(1)")

    Single citation: @einstein.

    Multiple keys: @dirac @bohr @planck.

    Reference: see Section @sec:intro and equation (@eq:emc).
    ==== WARNINGS ====
    []
    "#);
}

#[test]
fn m4_figure_bare_linewidth() {
    // Regression: `\includegraphics[width=\linewidth]{...}` with no numeric
    // coefficient must translate to `width: 100%`, not pass the LaTeX token
    // through verbatim (which Typst's parser rejects with `expected expression`).
    let src = "\\begin{figure}\n\
        \\centering\n\
        \\includegraphics[width=\\linewidth]{a.png}\n\
        \\caption{A}\n\
        \\end{figure}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("width: 100%"),
        "expected `width: 100%`, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\linewidth"),
        "raw `\\linewidth` should not leak into output:\n{}",
        out.typst
    );
}

#[test]
fn m4_figure_bare_textwidth() {
    let src = "\\includegraphics[width=\\textwidth]{a.png}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("width: 100%"),
        "expected `width: 100%`, got:\n{}",
        out.typst
    );
}

#[test]
fn m4_figure_bare_columnwidth() {
    let src = "\\includegraphics[width=\\columnwidth]{a.png}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("width: 100%"),
        "expected `width: 100%`, got:\n{}",
        out.typst
    );
}

#[test]
fn m4_newtheorem_dropped_silently() {
    // Regression: `\newtheorem*{remark}{Remark}` previously hit the generic
    // fallback and emitted the raw source into the Typst output, where the
    // leading backslash is invalid in code context. The tree-sitter-latex
    // grammar marks these as `theorem_definition`; the emitter now drops
    // that node kind alongside `\newcommand` / counter declarations.
    let src = "\\newtheorem{thm}{Theorem}\n\\newtheorem*{rem}{Remark}\n\nBody.\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("\\newtheorem"),
        "newtheorem definition should not leak into output, got:\n{}",
        out.typst
    );
    assert!(out.warnings.is_empty(), "got warnings: {:?}", out.warnings);
}

#[test]
fn m4_newtheorem_env_rendered() {
    // `\newtheorem{assumption}{Assumption}` should be harvested so that
    // `\begin{assumption}...\end{assumption}` is emitted as a theorem block
    // rather than producing an UnsupportedEnvironment warning.
    let src = concat!(
        "\\newtheorem{assumption}{Assumption}\n",
        "\\newtheorem*{rem}{Remark}\n\n",
        "\\begin{assumption}\\label{asm:main}\n",
        "The function is convex.\n",
        "\\end{assumption}\n\n",
        "\\begin{rem}\n",
        "This also holds for non-convex $f$.\n",
        "\\end{rem}\n",
    );
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.warnings.is_empty(),
        "expected no warnings, got: {:?}",
        out.warnings
    );
    assert!(
        out.typst.contains("kind: \"assumption\""),
        "expected #figure with kind:\"assumption\", got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("<asm:main>"),
        "expected label <asm:main>, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("kind: \"remark\""),
        "expected #figure with kind:\"remark\" (from \\newtheorem*{{rem}}{{Remark}}), got:\n{}",
        out.typst
    );
}

#[test]
fn m4_bibliography() {
    insta::assert_snapshot!(run("m4_floats/bibliography.tex"), @r#"
    ==== TYPST ====
    References are listed at the end.

    #bibliography("refs.bib", style: "ieee")
    ==== WARNINGS ====
    []
    "#);
}

// ============== Phase B: TDD red tests for Bugs #17, #19 ==============

#[test]
#[ignore = "Bug #17 — pending fix: unescaped _/*/#  in tabular cell content"]
fn m4_tabular_cell_escapes_markup_chars() {
    // Bug #17: `_sla`, `*bold`, `#hash` in table cells open unclosed italic /
    // bold / code-context markup in Typst. Cell content in `[...]` must have
    // these markup-active characters escaped with a backslash.
    let src = "\\begin{tabular}{cc}\n_sla & *bold \\\\\n\\#hash & plain\n\\end{tabular}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("[_sla]"),
        "raw `[_sla]` must not appear; `_` should be escaped, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("[*bold]"),
        "raw `[*bold]` must not appear; `*` should be escaped, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("\\_sla") || out.typst.contains("[\\_sla]"),
        "expected escaped `\\_sla`, got:\n{}",
        out.typst
    );
}

#[test]
#[ignore = "Bug #19 — pending fix: image(\"???\") placeholder fails typst compile"]
fn m4_figure_without_includegraphics_uses_compileable_placeholder() {
    // Bug #19: when a `\begin{figure}` has no `\includegraphics`, the emitter
    // produces `image("???")`. Typst aborts because the file `???` does not
    // exist. The fix should emit a compileable placeholder such as
    // `rect(width: 100%, height: 4cm, fill: luma(230))`.
    let src = "\\begin{figure}\n\\caption{X}\n\\end{figure}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        !out.typst.contains("image(\"???\")"),
        "broken placeholder `image(\"???\")` must not appear, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("rect(") || out.typst.contains("// missing"),
        "expected compileable placeholder (`rect(` or comment), got:\n{}",
        out.typst
    );
}

// ============== Phase D: under-tested emitters — tabular variants, nested figure ==============

#[test]
fn m4_tabular_star_with_width_argument() {
    // `tabular*` is a width-specified tabular; the width argument must be
    // consumed (not emitted verbatim). The column spec and cells should
    // render identically to a plain `tabular`.
    let src = "\\begin{tabular*}{0.9\\textwidth}{lcr}\na & b & c\n\\end{tabular*}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("columns: 3"),
        "expected `columns: 3`, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("left") && out.typst.contains("center") && out.typst.contains("right"),
        "expected lcr alignment in output, got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("tabular*"),
        "raw `tabular*` must not appear in output, got:\n{}",
        out.typst
    );
}

#[test]
fn m4_figure_wrapping_tabular_emits_inner_table() {
    // A `figure` environment containing a `tabular` should produce a
    // `#figure(table(...), caption: [...])` — the inner table must be emitted,
    // not silently dropped.
    let src = "\\begin{figure}\n\\begin{tabular}{cc}\n1 & 2\n\\end{tabular}\n\\caption{X}\n\\end{figure}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    assert!(
        out.typst.contains("table("),
        "expected `table(` inside figure, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("caption:"),
        "expected `caption:` in figure, got:\n{}",
        out.typst
    );
}

// ============== Bug B: nested math env no extra $ ==============

#[test]
fn m4_nested_math_env_no_extra_dollar() {
    // A math_environment that tree-sitter parses under an outer $...$
    // must NOT open a fresh `$ ... $` — that would close the outer math.
    let out = convert(
        "$\\Phi_{\\theta_t}(z(x,t))$",
        &ConvertOptions::default(),
    );
    let typst = &out.typst;
    // Count `$` signs: an opening $ and a closing $ is the minimum (2 total).
    // A spurious nested $ would give 4+.
    let dollar_count = typst.chars().filter(|&c| c == '$').count();
    assert!(
        dollar_count <= 2,
        "expected at most 2 dollar signs for inline math, got {}:\n{}",
        dollar_count,
        typst
    );
}

// ============== Bug D: unknown in-math command → valid placeholder ==============

#[test]
fn m4_unknown_math_command_no_raw_backslash() {
    // An unrecognised command inside math must NOT emit raw `\name` (which
    // Typst would error on) — instead emit a `"name"` string placeholder.
    let out = convert(r"$\diamT$", &ConvertOptions::default());
    assert!(
        !out.typst.contains('\\'),
        "expected no raw backslash in math output, got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains('"'),
        "expected quoted placeholder in math output, got:\n{}",
        out.typst
    );
    // Must produce at least one ambiguous_math warning.
    assert!(
        out.warnings.iter().any(|w| {
            serde_json::to_string(&w.category).unwrap_or_default().contains("ambiguous_math")
        }),
        "expected ambiguous_math warning, got:\n{:?}",
        out.warnings
    );
}
