//! `byetex diagnose` writes a diagnostics.json mapping each typst error to the
//! originating LaTeX fragment + skill. Gated on `typst` being available.
//!
//! Fixture rationale: `\cite{smith2020}` with no bibliography file causes
//! byetex to emit `@smith2020` which Typst rejects with
//! "label `<smith2020>` does not exist in the document".  This is a reliable
//! compile failure that byetex cannot suppress (it has no .bib to inline).
//! If this fixture ever stops failing (e.g. byetex starts emitting a stub
//! bibliography entry), replace it with another construct that fails typst.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn typst_available() -> bool {
    Command::new(std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".into()))
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

#[test]
fn diagnose_writes_diagnostics_json_with_mapped_error() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let dir = std::env::temp_dir().join(format!("byetex-diag-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // Fixture: \cite{smith2020} with no .bib file. Byetex emits `@smith2020`
    // which Typst rejects: "label `<smith2020>` does not exist in the document".
    // This reliably triggers at least one mapped error in the diagnostics output.
    let tex = dir.join("paper.tex");
    fs::write(
        &tex,
        "\\documentclass{article}\\begin{document}Some text~\\cite{smith2020}.\\end{document}\n",
    )
    .unwrap();

    let status = Command::new(bin())
        .arg("diagnose")
        .arg(&tex)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "diagnose should exit 0 even when the paper has errors"
    );

    let diag_path = dir.join("paper.diagnostics.json");
    let diag = fs::read_to_string(&diag_path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", diag_path.display()));
    let v: serde_json::Value = serde_json::from_str(&diag).unwrap();
    let arr = v.as_array().expect("diagnostics is a JSON array");
    assert!(!arr.is_empty(), "expected at least one mapped error, got: {diag}");
    let first = &arr[0];
    assert!(first.get("message").is_some());
    assert!(first.get("line").is_some());
    assert!(first.get("src_fragment").is_some()); // present (may be null)
    assert!(first.get("skill_name").is_some());

    let _ = fs::remove_dir_all(&dir);
}
