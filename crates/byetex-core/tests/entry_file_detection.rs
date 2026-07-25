//! `detect_entry_file` used to refuse any tree containing more than one
//! `\documentclass`, which is the normal shape of a real LaTeX repository (a
//! root `main.tex` plus an `examples/` or `templates/` directory). Review
//! finding #11: prefer the shallowest candidate, and only report an ambiguity
//! when the preference cannot single one out.

use byetex_core::project::{detect_entry_file, ProjectError};

fn tex(dir: &std::path::Path, rel: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(
        &p,
        "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n",
    )
    .unwrap();
}

#[test]
fn root_level_entry_wins_over_a_nested_example() {
    let tmp = tempfile::tempdir().unwrap();
    tex(tmp.path(), "main.tex");
    tex(tmp.path(), "examples/demo.tex");
    let entry = detect_entry_file(tmp.path()).expect("root main.tex should win");
    assert_eq!(entry.file_name().unwrap(), "main.tex", "picked {entry:?}");
}

#[test]
fn shallowest_candidate_wins_even_when_not_named_main() {
    let tmp = tempfile::tempdir().unwrap();
    tex(tmp.path(), "thesis.tex");
    tex(tmp.path(), "templates/a/sample.tex");
    tex(tmp.path(), "templates/b/sample.tex");
    let entry = detect_entry_file(tmp.path()).expect("shallowest should win");
    assert_eq!(entry.file_name().unwrap(), "thesis.tex", "picked {entry:?}");
}

#[test]
fn main_tex_breaks_a_same_depth_tie() {
    let tmp = tempfile::tempdir().unwrap();
    tex(tmp.path(), "main.tex");
    tex(tmp.path(), "supplement.tex");
    let entry = detect_entry_file(tmp.path()).expect("main.tex should break the tie");
    assert_eq!(entry.file_name().unwrap(), "main.tex", "picked {entry:?}");
}

#[test]
fn a_genuine_same_depth_ambiguity_is_still_reported() {
    let tmp = tempfile::tempdir().unwrap();
    tex(tmp.path(), "paper.tex");
    tex(tmp.path(), "poster.tex");
    match detect_entry_file(tmp.path()) {
        Err(ProjectError::AmbiguousEntryFile { candidates }) => {
            assert_eq!(candidates.len(), 2, "candidates: {candidates:?}");
        }
        other => panic!("expected AmbiguousEntryFile, got {other:?}"),
    }
}

#[test]
fn no_documentclass_anywhere_is_still_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("frag.tex"), "just text\n").unwrap();
    assert!(matches!(
        detect_entry_file(tmp.path()),
        Err(ProjectError::NoEntryFile { .. })
    ));
}
