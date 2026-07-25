//! The project pre-scan walk (`walk_project_files`) is shared by macro
//! harvesting, referenced-label harvesting, `\chapter` detection and entry-file
//! detection. It intended to skip symlinks but tested `file_type.is_dir()`
//! FIRST — and for a symlink-to-directory `symlink_metadata` reports neither
//! `is_dir()` nor `is_file()`, so the entry silently fell through both arms and
//! the whole subtree became invisible (review finding #10).
//!
//! A symlinked `common/` or `chapters/` directory is a normal way to share
//! sources between two papers, and `\input` resolution already reads straight
//! through such links, so the scan now follows them too — cycle-bounded.

#![cfg(unix)]

use byetex_core::project::plan_project_from_dir;

fn article(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn macros_under_a_symlinked_directory_are_harvested() {
    // The shared tree lives OUTSIDE the project; the symlink is the only route
    // to it, which is exactly the layout that silently lost its macros.
    let shared = tempfile::tempdir().unwrap();
    std::fs::write(
        shared.path().join("macros.tex"),
        "\\newcommand{\\projname}{ByeTex}\n",
    )
    .unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::os::unix::fs::symlink(shared.path(), root.join("common")).unwrap();
    std::fs::write(root.join("main.tex"), article("Hello \\projname.")).unwrap();

    let plan = plan_project_from_dir(root, true, false).expect("plan_project_from_dir");
    assert!(
        plan.main_typst.contains("ByeTex"),
        "macro defined under a symlinked directory was dropped:\n{}",
        plan.main_typst
    );
}

#[test]
fn labels_under_a_symlinked_directory_are_harvested() {
    let shared = tempfile::tempdir().unwrap();
    std::fs::write(
        shared.path().join("chapter1.tex"),
        "\\section{Intro}\\label{sec:intro}\n",
    )
    .unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::os::unix::fs::symlink(shared.path(), root.join("chapters")).unwrap();
    std::fs::write(
        root.join("main.tex"),
        article("\\input{chapters/chapter1}\nSee \\ref{sec:intro}."),
    )
    .unwrap();

    let plan = plan_project_from_dir(root, true, false).expect("plan_project_from_dir");
    assert!(
        plan.main_typst.contains("Intro"),
        "content behind a symlinked directory was dropped:\n{}",
        plan.main_typst
    );
}

#[test]
fn a_symlink_cycle_does_not_hang_the_walk() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("sub")).unwrap();
    // `sub/loop` points back at the project root.
    std::os::unix::fs::symlink(root, root.join("sub/loop")).unwrap();
    std::fs::write(root.join("main.tex"), article("x")).unwrap();

    let plan = plan_project_from_dir(root, true, false).expect("plan_project_from_dir");
    assert!(plan.main_typst.contains('x'), "conversion should complete");
}

#[test]
fn a_dangling_symlink_is_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::os::unix::fs::symlink(root.join("nowhere"), root.join("broken")).unwrap();
    std::fs::write(root.join("main.tex"), article("y")).unwrap();

    let plan = plan_project_from_dir(root, true, false).expect("plan_project_from_dir");
    assert!(plan.main_typst.contains('y'), "conversion should complete");
}
