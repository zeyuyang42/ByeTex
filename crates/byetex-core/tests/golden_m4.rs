//! M4 golden tests: tables, figures, citations, refs, bibliography.

use std::path::PathBuf;

use byetex_core::{convert, Category, ConvertOptions};

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

#[test]
fn m4_includegraphics_no_extension_resolves_with_extension() {
    // Bug #37 (fixed): `\includegraphics{foo}` (no extension) — when
    // `foo.png` exists on disk, the emitter used to write
    // `image("foo")` which Typst rejected with `file not found`
    // (Typst's `image()` requires the extension). The emitter now
    // probes for `foo.{png,pdf,jpg,jpeg,svg,gif}` and writes
    // `image("foo.png")` when it resolves.
    let tmp = std::env::temp_dir().join(format!(
        "byetex-img-ext-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    // Pretend a PNG exists.
    std::fs::write(tmp.join("plot.png"), b"fake png").unwrap();
    std::fs::write(
        tmp.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\n\\includegraphics{plot}\n\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("main.tex".into()),
        base_dir: Some(tmp.clone()),
    };
    let out = convert(&std::fs::read_to_string(tmp.join("main.tex")).unwrap(), &opts);
    assert!(
        out.typst.contains("image(\"plot.png\")"),
        "expected `image(\"plot.png\")` with resolved extension; got:\n{}",
        out.typst
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn m4_includegraphics_missing_file_emits_placeholder() {
    // Bug #37b (fixed): `\includegraphics{nowhere}` with no matching
    // file on disk used to still emit `image("nowhere")`, which
    // typst compile would abort on. The fallback now emits a
    // compileable `rect(...)` placeholder so the rest of the
    // document compiles.
    let tmp = std::env::temp_dir().join(format!(
        "byetex-img-missing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(
        tmp.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\n\\includegraphics{nowhere}\n\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("main.tex".into()),
        base_dir: Some(tmp.clone()),
    };
    let out = convert(&std::fs::read_to_string(tmp.join("main.tex")).unwrap(), &opts);
    assert!(
        !out.typst.contains("image(\"nowhere\""),
        "missing file should NOT keep raw `image(\"nowhere\")`; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("rect("),
        "expected placeholder `rect(...)`; got:\n{}",
        out.typst
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn m4_bibliography_drops_missing_files() {
    // Bug #27 (fixed): when `\bibliography{a,b,c}` lists multiple
    // files but only some are present in the source tree, Typst's
    // `#bibliography` aborts on the first missing file with `file
    // not found`. We now probe each listed `.bib` against the
    // base_dir and only emit the ones that resolve, warning about
    // the rest. Real driver: 2605.22776 bundles only `Stas.bib`
    // but `\bibliography` lists 4 paths.
    let tmp = std::env::temp_dir().join(format!(
        "byetex-bib-missing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    // Bundle only `present.bib`; reference both it and `missing.bib`.
    std::fs::write(tmp.join("present.bib"), "@article{x,title={X}}").unwrap();
    std::fs::write(
        tmp.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\nBody.\n\
         \\bibliography{present,missing}\n\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("main.tex".into()),
        base_dir: Some(tmp.clone()),
    };
    let out = convert(&std::fs::read_to_string(tmp.join("main.tex")).unwrap(), &opts);
    // The bibliography call must include `present.bib` (the file that
    // resolved) but NOT `missing.bib` (which would crash typst compile).
    assert!(
        out.typst.contains("present.bib"),
        "expected `present.bib` to survive in the output; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("missing.bib"),
        "missing.bib should be filtered out; got:\n{}",
        out.typst
    );
    // A needs_manual_review warning should flag the missing path.
    let has_missing_warning = out.warnings.iter().any(|w| {
        matches!(&w.category, Category::NeedsManualReview { reason }
            if reason.contains("missing.bib"))
    });
    assert!(
        has_missing_warning,
        "expected a needs_manual_review warning for missing.bib; got: {:?}",
        out.warnings
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

// ============== Phase B: TDD red tests for Bugs #17, #19 ==============

#[test]
fn m4_tabular_cell_escapes_markup_chars() {
    // Bug #17 (fixed): `_sla`, `*bold`, `#hash` in table cells used to open
    // unclosed italic / bold / code-context markup in Typst. Cell content in
    // `[...]` now has these markup-active characters escaped with a backslash.
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
fn m4_figure_without_includegraphics_uses_compileable_placeholder() {
    // Bug #19 (fixed): when a `\begin{figure}` had no `\includegraphics`, the
    // emitter produced `image("???")` and Typst aborted because the file
    // `???` did not exist. The fix emits a compileable placeholder (`rect(...)`
    // or a comment) instead.
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

#[test]
fn m4_tabular_row_split_preserves_escape_sequences() {
    // Bug #38 (fixed): the tabular emitter split rows on bare `\\`
    // (single backslash char) — which also matched `\\$`, `\\_`,
    // `\\*` etc. inside cell content. A `\\multicolumn{2}{c}{\\textbf{\\$10.23}}`
    // would fragment at every escape, corrupting the multicolumn
    // body to `*,\\n[$10.23\\*]` and breaking typst compile with
    // "unclosed delimiter".
    //
    // The splitter now uses `split_math_rows` (which distinguishes
    // `\\` followed by whitespace from `\\X` escape sequences),
    // mirroring the matrix/cases fix from Bug #31.
    let src = "\\begin{tabular}{cc}\na & \\multicolumn{2}{c}{\\textbf{\\$10.23}} \\\\\nb & c\n\\end{tabular}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    // The multicolumn cell body must contain the escaped dollar as
    // one unit, not get fragmented.
    assert!(
        out.typst.contains("table.cell(colspan: 2)[*\\$10.23*]"),
        "expected `table.cell(colspan: 2)[*\\$10.23*]` intact; got:\n{}",
        out.typst
    );
}

#[test]
fn m4_multicolumn_emits_complete_table_cell_expression() {
    // Bug #22 (fixed): `\multicolumn{N}{spec}{body}` used to emit
    // `table.cell(colspan: N)` with the body on a separate line —
    // Typst parsed that as a *call with no body argument*, breaking
    // the surrounding table. The fix sub-renders the body and writes
    // the single combined call `table.cell(colspan: N)[<body>]`.
    let src = "\\begin{tabular}{cccc}\n\\multicolumn{4}{c}{header text} \\\\\na & b & c & d\n\\end{tabular}\n";
    let out = convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    );
    // The colspan call must include its body in `[...]` directly.
    assert!(
        out.typst.contains("table.cell(colspan: 4)[header text]"),
        "expected `table.cell(colspan: 4)[header text]` in one expr; got:\n{}",
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
