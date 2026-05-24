//! M1 golden tests: paragraph passthrough on the `m1_passthrough/` fixtures.
//!
//! Uses inline `insta` snapshots so the expected output is committed alongside
//! the test. M1 promises: plain text and Unicode pass through unchanged, blank
//! lines remain paragraph separators, `%`-comments are dropped, no warnings.

use std::path::PathBuf;

use bytetex_core::{convert, ConvertOptions};


fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/bytetex-core has at least two parents")
        .join("tests/fixtures/m1_passthrough")
}

fn run_fixture(name: &str) -> String {
    let path = fixtures_dir().join(name);
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let opts = ConvertOptions {
        source_name: Some(name.to_string()),
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
fn m1_empty() {
    insta::assert_snapshot!(run_fixture("empty.tex"), @r"
    ==== TYPST ====
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m1_single_para() {
    insta::assert_snapshot!(run_fixture("single_para.tex"), @r"
    ==== TYPST ====
    Hello, world.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m1_hello() {
    insta::assert_snapshot!(run_fixture("hello.tex"), @r"
    ==== TYPST ====
    This is the first paragraph.

    This is the second paragraph; it spans
    two lines in the source.

    And here is the third.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m1_unicode() {
    insta::assert_snapshot!(run_fixture("unicode.tex"), @r"
    ==== TYPST ====
    Café résumé naïve.

    中文段落测试 — 你好，世界。

    Emoji-free: ø, å, ß, ñ.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m1_with_comments() {
    // LaTeX `%` swallows the rest of the line AND the following newline, so a
    // mid-line comment joins its surrounding lines with no break. The leading
    // comment line is similarly stripped (its own line vanishes).
    insta::assert_snapshot!(run_fixture("with_comments.tex"), @r"
    ==== TYPST ====
    The visible first paragraph.

    Second paragraph here. Continuing the second paragraph.
    ==== WARNINGS ====
    []
    ");
}

// ============ Text accent commands (Phase 2 of the silent-drop audit) ============

#[test]
fn m1_accent_brace_forms() {
    // Common braced accent forms → precomposed Unicode codepoints.
    // Uses r#"..."# because the input contains \" (diaeresis) which
    // would otherwise terminate a plain r"..." raw string.
    let out = convert(r#"\'{e} \`{a} \"{o} \^{i} \~{n}"#, &ConvertOptions::default());
    assert_eq!(out.typst.trim(), "é à ö î ñ");
    assert!(out.warnings.is_empty(), "unexpected warnings: {:?}", out.warnings);
}

#[test]
fn m1_accent_bare_forms() {
    // Bare form: the letter follows the command directly without braces.
    let out = convert(r"caf\'e", &ConvertOptions::default());
    assert_eq!(out.typst.trim(), "café");
    assert!(out.warnings.is_empty(), "unexpected warnings: {:?}", out.warnings);
}

#[test]
fn m1_accent_uppercase() {
    let out = convert(r#"\'{A} \"{O} \`{E}"#, &ConvertOptions::default());
    assert_eq!(out.typst.trim(), "Á Ö È");
    assert!(out.warnings.is_empty(), "unexpected warnings: {:?}", out.warnings);
}

// ============ ACM metadata capture (Phase 3 of the silent-drop audit) ============

#[test]
fn m1_acm_email_captured_in_metadata() {
    let src = r"\email{alice@example.edu}";
    let out = convert(src, &ConvertOptions::default());
    assert_eq!(
        out.class_metadata.get("email").map(String::as_str),
        Some("alice@example.edu"),
        "expected email in class_metadata, got: {:?}",
        out.class_metadata
    );
}

#[test]
fn m1_acm_authornote_captured_in_metadata() {
    let src = r"\authornote{Both authors contributed equally.}";
    let out = convert(src, &ConvertOptions::default());
    assert_eq!(
        out.class_metadata.get("authornote").map(String::as_str),
        Some("Both authors contributed equally."),
        "expected authornote in class_metadata, got: {:?}",
        out.class_metadata
    );
}
