//! `byetex convert --compile` runs a real `typst compile` and folds the result
//! into the agent brief (the reconciliation of the old `agent-brief` command).
//! Gated on `typst` being on PATH.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

fn typst_available() -> bool {
    Command::new(std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".into()))
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-cc-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn convert_without_compile_marks_brief_not_run() {
    let dir = tmpdir("no-compile");
    let tex = dir.join("p.tex");
    fs::write(&tex, "\\documentclass{article}\n\\begin{document}\nHi.\n\\end{document}\n").unwrap();
    let ok = Command::new(bin())
        .arg("convert")
        .arg(&tex)
        .status()
        .expect("run convert")
        .success();
    assert!(ok);
    let brief = fs::read_to_string(dir.join("p.agent_brief.md")).expect("brief written");
    assert!(
        brief.contains("(not run"),
        "plain convert must not run typst; got:\n{brief}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn convert_compile_runs_typst_into_brief() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let dir = tmpdir("compile");
    let tex = dir.join("p.tex");
    fs::write(&tex, "\\documentclass{article}\n\\begin{document}\nHi.\n\\end{document}\n").unwrap();
    let ok = Command::new(bin())
        .arg("convert")
        .arg(&tex)
        .arg("--compile")
        .status()
        .expect("run convert --compile")
        .success();
    assert!(ok);
    let brief = fs::read_to_string(dir.join("p.agent_brief.md")).expect("brief written");
    assert!(
        brief.contains("typst compile"),
        "the brief's Compile status must reflect a real typst run; got:\n{brief}"
    );
    assert!(
        !brief.contains("(not run"),
        "with --compile the brief must not say 'not run'; got:\n{brief}"
    );
    let _ = fs::remove_dir_all(&dir);
}
