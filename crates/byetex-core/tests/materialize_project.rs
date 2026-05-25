//! Integration tests for `byetex_core::project::materialize_project`.
//!
//! These tests moved from `byetex-cli/tests/project_materializer.rs` when
//! the materializer itself moved into byetex-core. They now exercise the
//! real `materialize_project` function instead of replicating its body in
//! the test (which the CLI-side tests had to do because the function used
//! to live in a private module of the binary crate).

use std::path::PathBuf;

use byetex_core::project::{materialize_project, plan_project};

fn fixture(rel: &str) -> PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    root.join("tests/fixtures").join(rel)
}

#[test]
fn materialize_writes_expected_tree() {
    let main = fixture("mini-project/main.tex");
    let base_dir = main.parent().unwrap().to_path_buf();
    let plan = plan_project(&main, true).expect("plan_project failed");

    let tmp = tempfile::TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");

    materialize_project(&plan, &out_dir, &base_dir, false).expect("materialize_project failed");

    // main.typ must exist.
    assert!(out_dir.join("main.typ").is_file(), "main.typ missing");

    // fig/a.pdf must be byte-for-byte identical to the fixture file.
    let dest_pdf = out_dir.join("fig/a.pdf");
    assert!(dest_pdf.is_file(), "fig/a.pdf missing from output");
    let src_pdf = fixture("mini-project/fig/a.pdf");
    assert_eq!(
        std::fs::read(&dest_pdf).unwrap(),
        std::fs::read(&src_pdf).unwrap(),
        "fig/a.pdf content mismatch"
    );

    // refs.bib must exist.
    let dest_bib = out_dir.join("refs.bib");
    assert!(dest_bib.is_file(), "refs.bib missing from output");
    let src_bib = fixture("mini-project/refs.bib");
    assert_eq!(
        std::fs::read(&dest_bib).unwrap(),
        std::fs::read(&src_bib).unwrap(),
        "refs.bib content mismatch"
    );

    // Verify intro.tex was NOT copied as a standalone asset.
    assert!(
        !out_dir.join("sections/intro.tex").is_file(),
        "intro.tex should not be a standalone asset copy"
    );
}

#[test]
fn materialize_path_traversal_guard_skips_escape() {
    let main = fixture("mini-project-escape/main.tex");
    let base_dir = main.parent().unwrap().to_path_buf();
    let plan = plan_project(&main, true).expect("plan_project failed");

    let tmp = tempfile::TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");

    materialize_project(&plan, &out_dir, &base_dir, false).expect("materialize_project failed");

    // The escape fixture references ../asset-discovery/fig/diagram which
    // is outside base_dir. The path-traversal guard must reject the copy.
    assert!(
        !out_dir.join("../asset-discovery/fig/diagram.pdf").exists(),
        "escaped asset should not be written"
    );
    // Output dir should only contain main.typ.
    let entries: Vec<_> = std::fs::read_dir(&out_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected only main.typ, got: {:?}",
        entries
    );
}

#[test]
fn materialize_refuses_non_empty_without_force() {
    let main = fixture("mini-project/main.tex");
    let base_dir = main.parent().unwrap().to_path_buf();
    let plan = plan_project(&main, true).expect("plan_project failed");

    let tmp = tempfile::TempDir::new().unwrap();
    let out_dir = tmp.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(out_dir.join("stale.txt"), b"leftover").unwrap();

    let result = materialize_project(&plan, &out_dir, &base_dir, false);
    assert!(
        result.is_err(),
        "expected refusal on non-empty dir without --force"
    );

    // With --force the stale file is wiped and main.typ takes its place.
    materialize_project(&plan, &out_dir, &base_dir, true)
        .expect("--force should succeed on non-empty dir");
    assert!(
        !out_dir.join("stale.txt").exists(),
        "force should clean stale entries"
    );
    assert!(out_dir.join("main.typ").is_file());
}
