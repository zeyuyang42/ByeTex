//! Integration tests for `byetex_core::compile`: compile a generated `.typ` to
//! a PDF with structured errors, and render it to per-page PNGs. The
//! compile/render paths need the real `typst` binary and skip cleanly when it
//! is absent (CI installs typst). `ensure_typ` needs no binary.

use std::path::Path;
use std::process::{Command, Stdio};

use byetex_core::compile::{compile_typ, ensure_typ, render_typ};

fn typst_available() -> bool {
    Command::new("typst")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn write(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn compile_valid_typ_reports_ok_and_writes_pdf() {
    if !typst_available() {
        eprintln!("skipping compile_render: `typst` not on PATH.");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let typ = write(dir.path(), "ok.typ", "Hello, world.\n");
    let res = compile_typ(&typ, None, "typst").unwrap();
    assert!(res.ok, "expected ok; errors={:?}", res.errors);
    assert!(
        res.errors.is_empty(),
        "no errors expected; got {:?}",
        res.errors
    );
    let pdf = res.pdf_path.expect("pdf path");
    assert!(Path::new(&pdf).exists(), "pdf should exist at {pdf}");
}

#[test]
fn compile_invalid_typ_reports_located_errors() {
    if !typst_available() {
        eprintln!("skipping compile_render: `typst` not on PATH.");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // A call to an undefined function → a typst error carrying a location.
    let typ = write(dir.path(), "bad.typ", "#this_is_undefined()\n");
    let res = compile_typ(&typ, None, "typst").unwrap();
    assert!(!res.ok, "expected compile failure");
    assert!(
        !res.errors.is_empty(),
        "expected at least one parsed error from typst stderr"
    );
    assert!(res.errors[0].line >= 1, "error should carry a 1-based line");
}

#[test]
fn render_two_page_typ_returns_ordered_pngs() {
    if !typst_available() {
        eprintln!("skipping compile_render: `typst` not on PATH.");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let typ = write(
        dir.path(),
        "two.typ",
        "#set page(height: 4cm)\nOne.\n#pagebreak()\nTwo.\n",
    );
    let out = dir.path().join("pages");
    let res = render_typ(&typ, &out, 80, "typst").unwrap();
    assert!(res.ok, "expected ok; errors={:?}", res.errors);
    assert_eq!(
        res.image_paths.len(),
        2,
        "two pages → two images: {:?}",
        res.image_paths
    );
    assert!(
        res.image_paths[0].ends_with("page-1.png"),
        "{:?}",
        res.image_paths
    );
    assert!(
        res.image_paths[1].ends_with("page-2.png"),
        "{:?}",
        res.image_paths
    );
    for p in &res.image_paths {
        assert!(Path::new(p).exists(), "image should exist: {p}");
    }
}

#[test]
fn ensure_typ_converts_tex_and_passes_through_typ() {
    let dir = tempfile::tempdir().unwrap();
    let tex = write(
        dir.path(),
        "p.tex",
        "\\documentclass{article}\\begin{document}Hi.\\end{document}\n",
    );
    let typ = ensure_typ(&tex).unwrap();
    assert_eq!(typ.extension().and_then(|s| s.to_str()), Some("typ"));
    assert!(typ.exists(), "converted .typ should be written");
    // An input that is already `.typ` passes through unchanged.
    let same = ensure_typ(&typ).unwrap();
    assert_eq!(same, typ);
}
