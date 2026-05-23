//! M4 golden tests: tables, figures, citations, refs, bibliography.

use std::path::PathBuf;

use bytetex_core::{convert, ConvertOptions};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/bytetex-core has at least two parents")
        .join("tests/fixtures")
}

fn run(rel: &str) -> String {
    let path = fixtures_root().join(rel);
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let opts = ConvertOptions {
        source_name: Some(rel.to_string()),
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
fn m4_bibliography() {
    insta::assert_snapshot!(run("m4_floats/bibliography.tex"), @r#"
    ==== TYPST ====
    References are listed at the end.

    #bibliography("refs.bib", style: "plain")
    ==== WARNINGS ====
    []
    "#);
}
