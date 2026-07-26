//! Converting a file must depend only on that file and what it `\input`s.
//!
//! `plan_project` pre-seeds the emitter with `\ref` targets so a reference in
//! one file can inform multi-label attachment for a section in another — but it
//! harvested them from EVERY `.tex` under the entry file's parent directory.
//! Since flat `byetex convert paper.tex` also routes through `plan_project`,
//! converting a paper in a directory that happens to hold other LaTeX projects
//! injected THEIR label keys as hidden anchors: an identical 14-line document
//! grew phantom `#hide[#figure([]) <label>]` lines it never referenced.
//!
//! That also broke the documented "same input, same output, every time"
//! guarantee — the output depended on unrelated neighbouring files.

use std::path::PathBuf;

use byetex_core::project::plan_project;

const PAPER: &str = "\\documentclass{article}\n\\begin{document}\n\\section{Hello}\nA short paper.\n\\end{document}\n";

/// A neighbouring, unrelated project that references labels of its own.
const NEIGHBOUR: &str = "\\documentclass{article}\n\\begin{document}\nSee \\ref{fig:alpha} and \\ref{tab:beta} and \\eqref{eq:gamma}.\n\\end{document}\n";

fn plan_typst(dir: &std::path::Path) -> String {
    plan_project(&dir.join("mine.tex"), true, false)
        .expect("plan_project")
        .main_typst
}

#[test]
fn an_unrelated_sibling_does_not_change_the_output() {
    let alone = tempfile::tempdir().unwrap();
    std::fs::write(alone.path().join("mine.tex"), PAPER).unwrap();
    let solo = plan_typst(alone.path());

    let shared = tempfile::tempdir().unwrap();
    std::fs::write(shared.path().join("mine.tex"), PAPER).unwrap();
    std::fs::write(shared.path().join("neighbour.tex"), NEIGHBOUR).unwrap();
    let with_sibling = plan_typst(shared.path());

    assert_eq!(
        solo, with_sibling,
        "the same input produced different output because of an unrelated \
         neighbouring file"
    );
}

#[test]
fn a_siblings_labels_do_not_leak_in_as_anchors() {
    let shared = tempfile::tempdir().unwrap();
    std::fs::write(shared.path().join("mine.tex"), PAPER).unwrap();
    std::fs::write(shared.path().join("neighbour.tex"), NEIGHBOUR).unwrap();
    let typst = plan_typst(shared.path());

    for key in ["fig:alpha", "tab:beta", "eq:gamma"] {
        assert!(
            !typst.contains(key),
            "label `{key}` from an unrelated sibling leaked into the output:\n{typst}"
        );
    }
}

/// The seeding exists for a real reason: a `\ref` in an `\input`'ed file must
/// still inform the labelled section it points at. That must keep working.
#[test]
fn labels_from_an_inputed_file_are_still_seeded() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("mine.tex"),
        "\\documentclass{article}\n\\begin{document}\n\\section{Results}\\label{sec:res}\\label{sec:alias}\n\\input{later}\n\\end{document}\n",
    )
    .unwrap();
    // The reference lives in a file the entry \input's — inside the project.
    std::fs::write(dir.path().join("later.tex"), "As shown in \\ref{sec:alias}.\n").unwrap();

    let typst = plan_typst(dir.path());
    assert!(
        typst.contains("sec:alias"),
        "a \\ref from an \\input'ed file must still reach the emitter:\n{typst}"
    );
}

/// Pointing at a DIRECTORY is an explicit statement that the whole tree is one
/// project, so the whole-tree harvest is correct there.
#[test]
fn directory_mode_still_harvests_the_whole_tree() {
    use byetex_core::project::plan_project_from_dir;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\n\\section{Results}\\label{sec:res}\\label{sec:alias}\nBody.\n\\end{document}\n",
    )
    .unwrap();
    // Not \input'ed, but in the same declared project directory.
    std::fs::write(dir.path().join("sibling.tex"), "See \\ref{sec:alias}.\n").unwrap();

    let plan = plan_project_from_dir(dir.path(), true, false).expect("plan_project_from_dir");
    assert!(
        plan.main_typst.contains("sec:alias"),
        "directory mode should still see the whole tree:\n{}",
        plan.main_typst
    );
}

#[test]
fn a_lone_file_in_a_crowded_directory_stays_small() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mine.tex"), PAPER).unwrap();
    for i in 0..5 {
        std::fs::write(
            dir.path().join(format!("other{i}.tex")),
            format!("\\documentclass{{article}}\n\\begin{{document}}\n\\ref{{junk:{i}}}\n\\end{{document}}\n"),
        )
        .unwrap();
    }
    let typst = plan_typst(dir.path());
    assert!(
        !typst.contains("junk:"),
        "unrelated projects in the same directory polluted the output:\n{}",
        PathBuf::from(&typst).display()
    );
}
