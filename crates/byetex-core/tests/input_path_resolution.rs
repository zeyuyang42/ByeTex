//! Regression tests for `\input{path}` resolution when the path contains
//! a directory component that is relative to the project ROOT rather than
//! the currently-processed file's directory.
//!
//! LaTeX resolves `\input` paths from the directory where the top-level
//! compiler was invoked (the project root). ByeTex was previously using
//! each sub-file's own directory as the resolution base, so
//! `\input{appendix/d_lemmas}` from inside `appendix/proofs.tex` would
//! look for `appendix/appendix/d_lemmas.tex` instead of the correct
//! `<root>/appendix/d_lemmas.tex`.

use std::path::{Path, PathBuf};

use byetex_core::{convert, ConvertOptions};

fn fixture_dir(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

/// Sets up a fixture directory with a root main.tex that inputs
/// `subdir/part.tex`, which in turn inputs `subdir/other.tex` via the
/// project-root-relative path `subdir/other` (as LaTeX convention allows).
fn create_nested_input_fixture() -> PathBuf {
    let base = fixture_dir("nested-input");
    std::fs::create_dir_all(base.join("subdir")).unwrap();

    // main.tex: inputs subdir/part.tex
    std::fs::write(
        base.join("main.tex"),
        b"\\documentclass{article}\n\\begin{document}\n\\input{subdir/part}\n\\end{document}\n",
    )
    .unwrap();

    // subdir/part.tex: inputs subdir/other via project-root-relative path
    std::fs::write(
        base.join("subdir/part.tex"),
        b"Part content here.\n\\input{subdir/other}\n",
    )
    .unwrap();

    // subdir/other.tex: defines a label
    std::fs::write(
        base.join("subdir/other.tex"),
        b"\\section{Other Section}\\label{sec:other}\nOther content.\n",
    )
    .unwrap();

    base
}

/// `\input{subdir/other}` from inside `subdir/part.tex` must resolve to
/// `<root>/subdir/other.tex`, not `<root>/subdir/subdir/other.tex`.
#[test]
fn nested_input_resolves_from_project_root() {
    let base = create_nested_input_fixture();
    let main_tex = base.join("main.tex");

    let src = std::fs::read_to_string(&main_tex).unwrap();
    let out = convert(
        &src,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(base.clone()),
        },
    );

    // If the path resolves correctly, "sec:other" label will be defined and
    // "Other content" will appear.
    assert!(
        out.typst.contains("Other content"),
        "subdir/other.tex was not included — nested \\input resolution failed;\
         got typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("<sec:other>"),
        "label from subdir/other.tex not present in output;\
         got typst:\n{}",
        out.typst
    );
    // Ensure no unresolved-include warning is present
    assert!(
        !out.warnings.iter().any(|w| w.message.contains("could not resolve")),
        "unexpected unresolved-include warning: {:?}",
        out.warnings
    );
}
