//! v0.2 golden snapshots: typography + title block.

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
        ..Default::default()
    };
    let out = convert(&source, &opts);
    format!("==== TYPST ====\n{}", out.typst)
}

#[test]
fn v02_typography_dashes_quotes() {
    insta::assert_snapshot!(run("v02_typography/dashes_quotes.tex"), @r#"
    ==== TYPST ====
    She said "hello" and the range 1–5 was 35—40 percent.

    Logos: LaTeX and BibTeX are by Knuth.
    "#);
}
