//! Layer B (compile check): convert each `*.tex` fixture and confirm the
//! emitted Typst compiles cleanly with the real `typst` CLI.
//!
//! If `typst` is not on `PATH`, the test prints a skip message and exits 0.
//! CI installs typst so the gate runs there; local dev users may not have it.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use byetex_core::{convert, ConvertOptions};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/byetex-core has at least two parents")
        .join("tests/fixtures")
}

fn typst_available() -> bool {
    Command::new("typst")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn try_compile_fixture(fixture: &str) -> Result<(), String> {
    let src_path = fixtures_root().join(fixture);
    let source = std::fs::read_to_string(&src_path)
        .map_err(|e| format!("read {}: {}", src_path.display(), e))?;
    let out = convert(&source, &ConvertOptions::default());

    let tmp = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let typ_path = tmp.path().join("out.typ");
    let pdf_path = tmp.path().join("out.pdf");
    std::fs::write(&typ_path, &out.typst)
        .map_err(|e| format!("write {}: {}", typ_path.display(), e))?;

    let result = Command::new("typst")
        .arg("compile")
        .arg(&typ_path)
        .arg(&pdf_path)
        .output()
        .map_err(|e| format!("invoke typst: {e}"))?;

    if !result.status.success() {
        return Err(format!(
            "typst compile of {fixture} failed (exit {}). stderr:\n{}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(())
}

#[test]
fn m1_fixtures_compile_to_pdf() {
    if !typst_available() {
        eprintln!(
            "skipping compile_check: `typst` not on PATH. Install it locally \
             or run this in CI where the setup-typst action installs it."
        );
        return;
    }
    let fixtures = [
        "m1_passthrough/empty.tex",
        "m1_passthrough/single_para.tex",
        "m1_passthrough/hello.tex",
        "m1_passthrough/unicode.tex",
        "m1_passthrough/with_comments.tex",
    ];
    for f in fixtures {
        try_compile_fixture(f).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn m3_fixtures_compile_to_pdf() {
    if !typst_available() {
        eprintln!("skipping compile_check: `typst` not on PATH.");
        return;
    }
    let fixtures = [
        "m3_math/inline_basic.tex",
        "m3_math/display_basic.tex",
        "m3_math/frac_sqrt.tex",
        "m3_math/greek_ops.tex",
        "m3_math/sum_int.tex",
        "m3_math/equation_env.tex",
        "m3_math/align_env.tex",
        "m3_math/pmatrix.tex",
    ];
    for f in fixtures {
        try_compile_fixture(f).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn m4_fixtures_compile_to_pdf() {
    if !typst_available() {
        eprintln!("skipping compile_check: `typst` not on PATH.");
        return;
    }
    // figure_basic uses `example.png` which doesn't exist; typst would warn
    // but for compile-check we just want a syntactically valid .typ. We can
    // skip figure_basic and confirm the rest.
    let fixtures = [
        "m4_floats/tabular_basic.tex",
        // m4_floats/cite_ref.tex references labels that don't exist in the
        // fixture (no `#bibliography` and no section/eq targets). Typst
        // refuses to compile dangling references; this is a deliberate Typst
        // behavior, not a converter bug.
        // m4_floats/bibliography.tex references refs.bib which doesn't exist locally.
        // m4_floats/figure_basic.tex references example.png which doesn't exist locally.
    ];
    for f in fixtures {
        try_compile_fixture(f).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn m2_fixtures_compile_to_pdf() {
    if !typst_available() {
        eprintln!("skipping compile_check: `typst` not on PATH.");
        return;
    }
    let fixtures = [
        // Sectioning
        "m2_sectioning/all_levels.tex",
        "m2_sectioning/starred.tex",
        "m2_sectioning/with_labels.tex",
        "m2_sectioning/mixed_body.tex",
        // Inline
        "m2_inline/basic.tex",
        "m2_inline/nested.tex",
        "m2_inline/in_heading.tex",
        // Lists
        "m2_lists/itemize.tex",
        "m2_lists/enumerate.tex",
        "m2_lists/description.tex",
        // Misc
        "m2_misc/linebreaks.tex",
        "m2_misc/full_article.tex",
    ];
    for f in fixtures {
        try_compile_fixture(f).unwrap_or_else(|e| panic!("{e}"));
    }
}
