//! The agent brief must orient a cold-start agent: a "Start here" pointer, the
//! diagnose-first repair loop with the don't-re-run rule, and a per-category
//! `→ <skill>` suffix so the agent knows which skill to read.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-brief-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn brief_has_start_here_repair_loop_and_skill_suffix() {
    let dir = tmpdir("content");
    let tex = dir.join("p.tex");
    // An unknown environment reliably yields an `unsupported_environment` warning
    // whose `suggested_skill` is `byetex-unsupported-environment`.
    fs::write(
        &tex,
        "\\documentclass{article}\n\\begin{document}\n\
         \\begin{unknownenvxyz}hi\\end{unknownenvxyz}\n\\end{document}\n",
    )
    .unwrap();
    let ok = Command::new(bin())
        .arg("convert")
        .arg(&tex)
        .status()
        .expect("run convert")
        .success();
    assert!(ok);
    let brief = fs::read_to_string(dir.join("p.agent_brief.md")).expect("brief written");

    assert!(
        brief.contains("Start here"),
        "brief must orient the agent with a Start here pointer; got:\n{brief}"
    );
    assert!(
        brief.contains("byetex-getting-started"),
        "Start here should point at the getting-started skill; got:\n{brief}"
    );
    assert!(
        brief.contains("re-scan AFTER edits") && brief.contains("IN PLACE"),
        "brief must tell the agent how to re-diagnose an edited .typ in place; got:\n{brief}"
    );
    assert!(
        brief.contains("byetex skills read byetex-repair-loop"),
        "brief must point at the repair-loop skill; got:\n{brief}"
    );
    assert!(
        brief.contains("→ byetex-unsupported-environment"),
        "warnings histogram must name the per-category skill; got:\n{brief}"
    );
    let _ = fs::remove_dir_all(&dir);
}
