//! `byetex review` looks for a cached reference ("truth") PDF beside the
//! source. It used to take the FIRST `.pdf` `read_dir` handed back, so a paper
//! whose source directory ships figure PDFs (8 of the 59 corpus papers do) was
//! graded against a one-page plot — and non-deterministically, since `read_dir`
//! order is unspecified (review finding #6).
//!
//! Only `<entry-stem>.pdf` is a plausible cached render of the entry file.

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
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

fn packet(dir: &std::path::Path, tex: &std::path::Path) -> serde_json::Value {
    let out = dir.join("review-out");
    let status = Command::new(bin())
        .arg("review")
        .arg(tex)
        .arg("--out")
        .arg(&out)
        // Force tectonic absent: with no usable cached PDF the truth must
        // resolve to "none" rather than to some unrelated figure.
        .env("BYETEX_TECTONIC_BIN", "byetex-tectonic-does-not-exist-xyz")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run byetex review");
    assert!(status.success(), "review exited {status:?}");
    serde_json::from_str(&std::fs::read_to_string(out.join("grading_packet.json")).unwrap())
        .expect("valid packet json")
}

#[test]
fn a_stray_figure_pdf_is_not_mistaken_for_the_truth_render() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let tex = tmp.path().join("paper.tex");
    std::fs::write(
        &tex,
        "\\documentclass{article}\\begin{document}Hello, fidelity.\\end{document}\n",
    )
    .unwrap();
    // A figure PDF sitting at the root of the source directory — the shape of
    // corpus 2605.31011 / 2605.22779.
    std::fs::write(
        tmp.path().join("architecture-diagram.pdf"),
        b"%PDF-1.4 not the paper",
    )
    .unwrap();

    let v = packet(tmp.path(), &tex);
    assert_eq!(
        v["truth_source"], "none",
        "a stray figure PDF was used as the truth render: {v}"
    );
}

#[test]
fn a_cached_render_of_the_entry_file_is_still_used() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let tex = tmp.path().join("paper.tex");
    std::fs::write(
        &tex,
        "\\documentclass{article}\\begin{document}Hello, fidelity.\\end{document}\n",
    )
    .unwrap();
    std::fs::write(tmp.path().join("paper.pdf"), b"%PDF-1.4 cached render").unwrap();

    let v = packet(tmp.path(), &tex);
    assert_eq!(
        v["truth_source"], "cached",
        "`<entry-stem>.pdf` should still be honoured: {v}"
    );
}
