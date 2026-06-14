//! Integration test for `byetex review`: it builds a `grading_packet.json` for
//! a paper, rendering the Typst side to per-page PNGs. Needs `typst` (skips if
//! absent). Truth is best-effort, so this test forces tectonic absent — making
//! the truth-less path deterministic — and asserts the typst side + a
//! well-formed packet.

use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_byetex")
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

#[test]
fn review_builds_packet_with_typst_pages() {
    if !typst_available() {
        eprintln!("skipping review test: `typst` not on PATH.");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tex = dir.path().join("p.tex");
    std::fs::write(
        &tex,
        "\\documentclass{article}\\begin{document}Hello, fidelity.\\end{document}\n",
    )
    .unwrap();
    let out = dir.path().join("review");

    let status = Command::new(bin())
        .arg("review")
        .arg(&tex)
        .arg("--out")
        .arg(&out)
        // Force tectonic absent so the truth path is deterministically "none"
        // (no cached PDF in the tempdir either) — the test stays hermetic.
        .env("BYETEX_TECTONIC_BIN", "byetex-tectonic-does-not-exist-xyz")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run byetex review");
    assert!(status.success(), "review should exit 0; got {status:?}");

    let packet = out.join("grading_packet.json");
    assert!(packet.is_file(), "packet missing at {}", packet.display());
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&packet).unwrap()).expect("valid packet json");

    assert_eq!(v["detected_class"], "article", "class: {v}");
    assert_eq!(v["truth_source"], "none", "truth should be none: {v}");
    let pages = v["pages"].as_array().expect("pages array");
    assert!(!pages.is_empty(), "expected at least one page: {v}");
    assert!(
        pages[0]["typst"].is_string(),
        "first page must have a typst image: {v}"
    );
    assert_eq!(v["rubric"], "docs/fidelity-rubric.md");
}
