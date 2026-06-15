//! Tests for the folder-input path: `detect_entry_file`,
//! `plan_project_from_dir`, and the project-wide macro pre-scan.

use std::fs;
use std::path::{Path, PathBuf};

use byetex_core::project::{detect_entry_file, plan_project_from_dir, ProjectError};

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-folder-{}-{}", name, std::process::id()));
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
fn detect_entry_file_picks_the_single_documentclass() {
    let dir = tmpdir("detect-single");
    write(
        &dir,
        "paper.tex",
        "\\documentclass{article}\n\\begin{document}\nHi\n\\end{document}\n",
    );
    write(&dir, "sections/intro.tex", "Hello world.\n");

    let entry = detect_entry_file(&dir).expect("entry should be found");
    assert!(entry.ends_with("paper.tex"));
}

#[test]
fn detect_entry_file_ignores_commented_documentclass() {
    let dir = tmpdir("detect-commented");
    write(
        &dir,
        "real.tex",
        "\\documentclass{article}\n\\begin{document}\nx\\end{document}\n",
    );
    write(
        &dir,
        "notes.tex",
        "% This file used to start with \\documentclass{article}\nrandom prose\n",
    );

    let entry = detect_entry_file(&dir).unwrap();
    assert!(entry.ends_with("real.tex"));
}

#[test]
fn detect_entry_file_errors_on_zero_candidates() {
    let dir = tmpdir("detect-none");
    write(&dir, "scratch.tex", "no documentclass here\n");
    write(&dir, "README.md", "not a tex file\n");

    match detect_entry_file(&dir) {
        Err(ProjectError::NoEntryFile { .. }) => {}
        other => panic!("expected NoEntryFile, got {:?}", other),
    }
}

#[test]
fn detect_entry_file_errors_on_multiple_candidates() {
    let dir = tmpdir("detect-many");
    write(&dir, "paper-a.tex", "\\documentclass{article}\nA\n");
    write(&dir, "paper-b.tex", "\\documentclass{report}\nB\n");

    match detect_entry_file(&dir) {
        Err(ProjectError::AmbiguousEntryFile { candidates }) => {
            assert_eq!(candidates.len(), 2);
        }
        other => panic!("expected AmbiguousEntryFile, got {:?}", other),
    }
}

#[test]
fn detect_entry_file_skips_hidden_dirs() {
    let dir = tmpdir("detect-hidden");
    write(&dir, "main.tex", "\\documentclass{article}\n");
    // A dotfile-prefixed dir (e.g. .git, .vscode) must not contribute.
    write(&dir, ".trash/old.tex", "\\documentclass{report}\n");

    let entry = detect_entry_file(&dir).unwrap();
    assert!(entry.ends_with("main.tex"));
}

#[test]
fn plan_project_from_dir_uses_sibling_sty_macros() {
    // The entry never `\input`s mystyle.sty, but a project-wide
    // pre-scan should harvest \brand and expand it at the call site.
    let dir = tmpdir("preseed-sty");
    write(&dir, "mystyle.sty", "\\newcommand{\\brand}{ByeTex}\n");
    write(
        &dir,
        "paper.tex",
        "\\documentclass{article}\n\\begin{document}\nHello \\brand!\n\\end{document}\n",
    );

    let plan = plan_project_from_dir(&dir, true, false).expect("plan");
    assert!(
        plan.main_typst.contains("ByeTex"),
        "expected `\\brand` to expand to ByeTex; main.typ was:\n{}",
        plan.main_typst
    );
    assert!(plan.manifest.is_none(), "no_toml requested");
}

#[test]
fn plan_project_from_dir_preserves_entry_at_subpath() {
    // Many arXiv tarballs nest sources under e.g. `latex_source/`.
    let dir = tmpdir("nested-entry");
    write(
        &dir,
        "latex_source/main.tex",
        "\\documentclass{article}\n\\begin{document}\ncontent\n\\end{document}\n",
    );

    let entry = detect_entry_file(&dir).unwrap();
    assert!(
        entry.ends_with("latex_source/main.tex"),
        "entry should be the nested file, got {}",
        entry.display()
    );
    let plan = plan_project_from_dir(&dir, true, false).unwrap();
    assert!(plan.main_typst.contains("content"));
}

#[test]
fn wrapper_newcommand_in_sty_is_harvested_via_dir_mode() {
    // Regression for wrapper-macro pattern: a .sty defines macros using
    // a wrapper (\mytoken[2]{\newcommand{#1}{body}}), and calls like
    // \mytoken{\token}{t} are scattered through the project files.
    // harvest_project_macros must expand these calls so \token reaches
    // the emitter's macro table.
    let dir = tmpdir("wrapper-harvest");
    write(
        &dir,
        "macros.sty",
        concat!(
            "\\newcommand{\\mytoken}[2]{\\newcommand{#1}{{#2}}}\n",
            "\\mytoken{\\token}{t}\n",
            "\\mytoken{\\vocab}{\\mathcal{T}}\n",
        ),
    );
    write(
        &dir,
        "main.tex",
        concat!(
            "\\documentclass{article}\n",
            "\\usepackage{macros}\n",
            "\\begin{document}\n",
            "Token $\\token$ vocab $\\vocab$\n",
            "\\end{document}\n",
        ),
    );

    let plan = byetex_core::project::plan_project_from_dir(&dir, false, false)
        .expect("plan_project_from_dir");
    let ambiguous_count = plan
        .warnings
        .iter()
        .filter(|w| format!("{:?}", w.category).contains("ambiguous_math"))
        .count();
    assert_eq!(
        ambiguous_count,
        0,
        "\\token and \\vocab should expand via wrapper-newcommand harvest; warnings: {:?}",
        plan.warnings
            .iter()
            .map(|w| format!("{:?}: {}", w.category, w.snippet))
            .collect::<Vec<_>>()
    );
}

#[test]
fn wrapper_newcommand_calls_in_main_tex_are_harvested() {
    // Same-file case: wrapper definitions AND calls both in main.tex.
    // This is the 22821 structure (no separate macros.sty).
    let dir = tmpdir("wrapper-same-file");
    write(
        &dir,
        "main.tex",
        concat!(
            "\\documentclass{article}\n",
            "\\newcommand{\\mytoken}[2]{\\newcommand{#1}{{\\color{x}#2}}}\n",
            "\\mytoken{\\token}{t}\n",
            "\\mytoken{\\vocab}{\\mathcal{T}}\n",
            "\\begin{document}\n",
            "Token $\\token$ vocab $\\vocab$\n",
            "\\end{document}\n",
        ),
    );
    let plan = byetex_core::project::plan_project_from_dir(&dir, false, false)
        .expect("plan_project_from_dir");
    let ambiguous_count = plan
        .warnings
        .iter()
        .filter(|w| format!("{:?}", w.category).contains("ambiguous_math"))
        .count();
    assert_eq!(
        ambiguous_count,
        0,
        "\\token and \\vocab should expand; warnings: {:?}",
        plan.warnings
            .iter()
            .map(|w| format!("{:?}: {}", w.category, w.snippet))
            .collect::<Vec<_>>()
    );
}
