//! `render_typ` globs every `page-<N>.png` in its output directory after the
//! typst run, so pages left over from a PREVIOUS render of a longer document
//! were reported as current output — inflating the page count fed to the visual
//! grading packet (review finding #7).

use std::path::Path;
use std::process::{Command, Stdio};

fn typst_bin() -> String {
    std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".to_string())
}

fn typst_available() -> bool {
    Command::new(typst_bin())
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn render_clears_stale_page_pngs_from_a_previous_run() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let typ = tmp.path().join("doc.typ");
    std::fs::write(&typ, "Just one page.\n").unwrap();
    let out_dir = tmp.path().join("pages");
    std::fs::create_dir_all(&out_dir).unwrap();

    // Leftovers from an earlier, longer document.
    for n in 1..=4 {
        std::fs::write(out_dir.join(format!("page-{n}.png")), b"stale").unwrap();
    }
    // A non-page PNG the caller put there must survive.
    std::fs::write(out_dir.join("truth-1.png"), b"keep").unwrap();

    let r = byetex_core::compile::render_typ(&typ, &out_dir, 50, &typst_bin()).expect("render_typ");
    assert!(r.ok, "typst render failed: {:?}", r.errors);
    assert_eq!(
        r.image_paths.len(),
        1,
        "stale pages counted as output: {:?}",
        r.image_paths
    );
    assert!(
        Path::new(&out_dir.join("truth-1.png")).exists(),
        "unrelated PNGs must not be deleted"
    );
}
