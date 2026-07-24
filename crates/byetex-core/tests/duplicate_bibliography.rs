//! A document that calls `\bibliography{refs}` more than once (common when a
//! `\clearpage\bibliography{...}` survives alongside a class-provided one) used
//! to produce an output that could not compile at all:
//!
//!   * two `#bibliography(...)` calls — Typst hard-errors with
//!     "multiple bibliographies are not yet supported" (review finding #4);
//!   * the same `.bib` registered twice as an asset, and because the
//!     `@key`-dedup state is shared across `.bib` files, the SECOND
//!     materialisation of the same destination wrote an EMPTY file over the
//!     good first one (review finding #3).

use byetex_core::project::{materialize_project, plan_project};

const BIB: &str = "@article{smith2020,\n  title = {A Title},\n  author = {Smith, J.},\n  year = {2020},\n  journal = {J. Test}\n}\n";

fn write_project(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
    std::fs::write(dir.join("refs.bib"), BIB).unwrap();
    let main = dir.join("main.tex");
    std::fs::write(
        &main,
        format!(
            "\\documentclass{{article}}\n\\begin{{document}}\nText~\\cite{{smith2020}}.\n{body}\\end{{document}}\n"
        ),
    )
    .unwrap();
    main
}

#[test]
fn repeated_bibliography_emits_a_single_typst_call() {
    let tmp = tempfile::tempdir().unwrap();
    let main = write_project(
        tmp.path(),
        "\\bibliography{refs}\n\\bibliographystyle{plain}\n\\bibliography{refs}\n",
    );
    let plan = plan_project(&main, true, false).expect("plan_project");
    let n = plan.main_typst.matches("#bibliography(").count();
    assert_eq!(
        n, 1,
        "expected exactly one #bibliography call, got {n}:\n{}",
        plan.main_typst
    );
}

#[test]
fn repeated_bibliography_materializes_a_non_empty_bib() {
    let tmp = tempfile::tempdir().unwrap();
    let main = write_project(
        tmp.path(),
        "\\bibliography{refs}\n\\bibliography{refs}\n",
    );
    let plan = plan_project(&main, true, false).expect("plan_project");

    let out = tempfile::tempdir().unwrap();
    let out_dir = out.path().join("proj");
    materialize_project(&plan, &out_dir, tmp.path(), false).expect("materialize_project");

    let dest = out_dir.join("refs.bib");
    let text = std::fs::read_to_string(&dest).expect("refs.bib materialized");
    assert!(
        text.contains("smith2020"),
        "the duplicate .bib asset overwrote the good copy with an empty file (len {}): {:?}",
        text.len(),
        text
    );
}

#[test]
fn distinct_bib_files_still_dedupe_shared_keys() {
    // Guard the behaviour the shared `seen_keys` set exists for: two DIFFERENT
    // .bib files listing the same key must still emit it once.
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.bib"), BIB).unwrap();
    std::fs::write(tmp.path().join("b.bib"), BIB).unwrap();
    let main = tmp.path().join("main.tex");
    std::fs::write(
        &main,
        "\\documentclass{article}\n\\begin{document}\nText~\\cite{smith2020}.\n\\bibliography{a,b}\n\\end{document}\n",
    )
    .unwrap();
    let plan = plan_project(&main, true, false).expect("plan_project");

    let out = tempfile::tempdir().unwrap();
    let out_dir = out.path().join("proj");
    materialize_project(&plan, &out_dir, tmp.path(), false).expect("materialize_project");

    let a = std::fs::read_to_string(out_dir.join("a.bib")).unwrap();
    let b = std::fs::read_to_string(out_dir.join("b.bib")).unwrap();
    assert!(a.contains("smith2020"), "first .bib should keep the entry");
    assert!(
        !b.contains("smith2020"),
        "duplicate key across DIFFERENT files must still be dropped from the second: {b:?}"
    );
}
