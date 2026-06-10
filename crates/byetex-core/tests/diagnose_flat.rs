//! End-to-end test for the shared `byetex_core::diagnose::diagnose_flat`
//! orchestration (convert → typst compile → map). This is the function the CLI
//! `diagnose` command and the MCP `diagnose` tool both call. Gated on `typst`.
//!
//! Fixture: `\cite{smith2020}` with no bibliography → byetex emits `@smith2020`,
//! which typst rejects with "label `<smith2020>` does not exist" — a reliable
//! compile failure byetex cannot suppress (no `.bib` to inline).

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

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-diagflat-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn diagnose_flat_maps_a_real_typst_error() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let dir = tmpdir("maps-error");
    let tex = dir.join("paper.tex");
    fs::write(
        &tex,
        "\\documentclass{article}\n\\begin{document}\nSee \\cite{smith2020}.\n\\end{document}\n",
    )
    .unwrap();

    let typst = std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".into());
    let (typ_path, diags) =
        byetex_core::diagnose::diagnose_flat(&tex, None, &typst).expect("diagnose_flat");

    assert!(typ_path.exists(), "the .typ was written");
    assert!(!diags.is_empty(), "expected at least one diagnostic; got {diags:?}");
    assert!(
        diags.iter().any(|d| d.message.contains("smith2020")),
        "expected the dangling-cite error; got {diags:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}
