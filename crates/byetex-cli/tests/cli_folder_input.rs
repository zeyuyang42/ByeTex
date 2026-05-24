//! End-to-end CLI smoke test for folder input.
//!
//! Builds the `byetex` binary via cargo and runs it against a temp dir
//! that simulates an arXiv-style layout (entry .tex + a sibling .sty
//! whose macro is never `\input`ed). The expected outcome is a
//! self-contained typst-project directory with main.typ that contains
//! the expanded macro.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary_path() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by cargo for integration tests so the
    // test always picks up the freshly-built binary in target/.
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

fn tmpdir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("byetex-cli-folder-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn cli_convert_project_accepts_a_directory_and_pre_scans_macros() {
    let project = tmpdir("convert-project-dir");
    write(
        &project,
        "macros.sty",
        "\\newcommand{\\brand}{ByeTex}\n",
    );
    write(
        &project,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\nHello \\brand!\n\\end{document}\n",
    );

    let out_dir = project.with_extension("typst-project");
    let _ = fs::remove_dir_all(&out_dir);

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&project)
        .arg("--project")
        .arg("--project-out")
        .arg(&out_dir)
        .arg("--no-toml")
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    let main_typ = out_dir.join("main.typ");
    assert!(main_typ.is_file(), "main.typ was not written");
    let body = fs::read_to_string(&main_typ).unwrap();
    assert!(
        body.contains("ByeTex"),
        "expected pre-scanned \\brand macro to expand; main.typ:\n{}",
        body
    );

    let warnings = out_dir.join("warnings.json");
    assert!(warnings.is_file(), "warnings.json was not written");
}

#[test]
fn braceless_user_macros_dont_warn() {
    // Real arXiv papers (corpus/online/arxiv/paper) heavily use
    // brace-less calls like `$\mat X$` for 1-arg `\newcommand`s. The
    // pre-scan harvests the definition from a sibling `.tex`; the
    // expander must accept the brace-less call form.
    let project = tmpdir("braceless-macros");
    write(
        &project,
        "macros.tex",
        "\\newcommand{\\mat}[1]{\\mathbf{#1}}\n\
         \\newcommand{\\rvec}[1]{\\mathbf{#1}}\n",
    );
    write(
        &project,
        "main.tex",
        "\\documentclass{article}\n\
         \\input{macros.tex}\n\
         \\begin{document}\n\
         The matrix $\\mat X$ and the vector $\\rvec y$ are scary.\n\
         \\end{document}\n",
    );

    let out_dir = project.with_extension("typst-project");
    let _ = fs::remove_dir_all(&out_dir);

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&project)
        .arg("--project")
        .arg("--project-out")
        .arg(&out_dir)
        .arg("--no-toml")
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    let warnings_text = fs::read_to_string(out_dir.join("warnings.json")).unwrap();
    let warnings: serde_json::Value = serde_json::from_str(&warnings_text).unwrap();
    let custom_macro_count = warnings
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| {
            w.get("category")
                .and_then(|c| c.get("kind"))
                .and_then(|s| s.as_str())
                == Some("custom_macro")
        })
        .count();
    assert_eq!(
        custom_macro_count, 0,
        "expected zero custom_macro warnings; got {}: {}",
        custom_macro_count, warnings_text
    );

    let body = fs::read_to_string(out_dir.join("main.typ")).unwrap();
    assert!(
        body.contains("bold(X)") || body.contains("bold( X )"),
        "expected `\\mat X` to expand to bold(X); main.typ:\n{}",
        body
    );
}

#[test]
fn cli_convert_dir_without_project_writes_flat_typ() {
    let project = tmpdir("convert-flat-dir");
    write(
        &project,
        "paper.tex",
        "\\documentclass{article}\n\\begin{document}\ncontent\n\\end{document}\n",
    );

    let status = Command::new(binary_path())
        .arg("convert")
        .arg(&project)
        .status()
        .expect("running byetex");
    assert!(status.success(), "byetex exited with {:?}", status);

    // Default flat output: `<dirname>.typ` next to the dir.
    let dir_name = project.file_name().unwrap().to_str().unwrap();
    let parent = project.parent().unwrap();
    let flat = parent.join(format!("{}.typ", dir_name));
    assert!(flat.is_file(), "expected flat output at {}", flat.display());
    let warnings = parent.join(format!("{}.warnings.json", dir_name));
    assert!(warnings.is_file(), "expected warnings sidecar");
}
