//! Tests for `\input{...}` / `\include{...}` inline expansion.
//!
//! When `ConvertOptions::base_dir` is set, ByeTex resolves the include
//! relative to that directory and recursively converts the referenced file
//! so its body appears inline. Without `base_dir` set, the directive is
//! still dropped with a `needs_manual_review` warning (the v0.1 behaviour
//! for callers that pass raw source strings with no containing file).

use std::fs;
use std::path::PathBuf;

use bytetex_core::{convert, Category, ConvertOptions};
use tempfile::TempDir;

fn run_with_base(main: &str, base: PathBuf) -> bytetex_core::ConvertOutput {
    convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(base),
        },
    )
}

#[test]
fn input_expands_inline_when_base_dir_set() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("body.tex"),
        "Hello from the included file.\n",
    )
    .unwrap();
    let main = "Before.\n\n\\input{body}\n\nAfter.\n";
    let out = run_with_base(main, tmp.path().to_path_buf());
    assert!(
        out.typst.contains("Hello from the included file."),
        "expected include body to appear inline; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("Before."),
        "parent body before include should still appear; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("After."),
        "parent body after include should still appear; got:\n{}",
        out.typst
    );
    assert!(
        out.warnings.is_empty(),
        "successful expansion should not warn; got: {:?}",
        out.warnings
    );
}

#[test]
fn input_resolves_with_implicit_tex_extension() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(tmp.path().join("intro.tex"), "Intro body.\n").unwrap();
    // No `.tex` in the directive — LaTeX appends it automatically.
    let main = "\\input{intro}\n";
    let out = run_with_base(main, tmp.path().to_path_buf());
    assert!(out.typst.contains("Intro body."));
    assert!(out.warnings.is_empty(), "got: {:?}", out.warnings);
}

#[test]
fn include_resolves_with_subdirectory_path() {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("sections")).unwrap();
    fs::write(tmp.path().join("sections/one.tex"), "Section one.\n").unwrap();
    let main = "\\include{sections/one}\n";
    let out = run_with_base(main, tmp.path().to_path_buf());
    assert!(out.typst.contains("Section one."), "got: {}", out.typst);
}

#[test]
fn nested_input_resolves_relative_to_includer() {
    // a.tex \input{b}; b.tex \input{c}; c.tex is leaf
    let tmp = TempDir::new().expect("tempdir");
    fs::write(tmp.path().join("a.tex"), "Top.\n\\input{b}\n").unwrap();
    fs::write(tmp.path().join("b.tex"), "Middle.\n\\input{c}\n").unwrap();
    fs::write(tmp.path().join("c.tex"), "Bottom.\n").unwrap();
    let main = fs::read_to_string(tmp.path().join("a.tex")).unwrap();
    let out = run_with_base(main.as_str(), tmp.path().to_path_buf());
    assert!(out.typst.contains("Top."));
    assert!(out.typst.contains("Middle."));
    assert!(out.typst.contains("Bottom."));
    assert!(out.warnings.is_empty(), "got: {:?}", out.warnings);
}

#[test]
fn missing_input_warns_with_needs_manual_review() {
    let tmp = TempDir::new().expect("tempdir");
    let main = "\\input{does_not_exist}\n";
    let out = run_with_base(main, tmp.path().to_path_buf());
    assert_eq!(out.warnings.len(), 1, "got: {:?}", out.warnings);
    assert!(matches!(
        out.warnings[0].category,
        Category::NeedsManualReview { .. }
    ));
}

#[test]
fn circular_input_does_not_loop() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(tmp.path().join("a.tex"), "A.\n\\input{b}\n").unwrap();
    fs::write(tmp.path().join("b.tex"), "B.\n\\input{a}\n").unwrap();
    let main = fs::read_to_string(tmp.path().join("a.tex")).unwrap();
    let out = run_with_base(main.as_str(), tmp.path().to_path_buf());
    // Body from both should appear once (or for `a` from main + once
    // before the cycle break) and one cycle warning should fire.
    assert!(out.typst.contains("A."));
    assert!(out.typst.contains("B."));
    let cycle_warnings = out
        .warnings
        .iter()
        .filter(|w| matches!(&w.category, Category::NeedsManualReview { reason } if reason.contains("circular")))
        .count();
    assert!(
        cycle_warnings >= 1,
        "expected at least one circular-include warning; got: {:?}",
        out.warnings
    );
}

#[test]
fn without_base_dir_input_still_warns() {
    // Backward-compatible fallback: when no base_dir is given, the
    // directive is dropped with a needs_manual_review warning as in v0.1.
    let out = convert(
        "\\input{anything}\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            base_dir: None,
        },
    );
    assert_eq!(out.warnings.len(), 1);
    assert!(matches!(
        out.warnings[0].category,
        Category::NeedsManualReview { .. }
    ));
}

#[test]
fn included_title_propagates_to_parent() {
    // An \input that contains \title{...} should still produce a title in
    // the parent's preamble (typical pattern: a "header.tex" with title +
    // author block, included from main.tex).
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("header.tex"),
        "\\title{Included Title}\n\\author{X}\n",
    )
    .unwrap();
    let main = "\\input{header}\n\\maketitle\n\nBody.\n";
    let out = run_with_base(main, tmp.path().to_path_buf());
    assert!(
        out.typst.contains("Included Title"),
        "title from include should reach the parent's title block; got:\n{}",
        out.typst
    );
}
