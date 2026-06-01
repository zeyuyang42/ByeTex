//! Phase 2c / defect D6: `\includegraphics` paths resolve relative to the MAIN
//! document directory, not the `\input`-ed file's directory. A figure
//! `\includegraphics{figures/x.png}` inside `appendix/foo.tex` lives at
//! `<root>/figures/x.png`. ByeTex resolved only against the included file's dir
//! (`<root>/appendix/figures/x.png`), so every figure in an `\input`-ed file
//! resolved as "missing" (corpus 2605.22765 / 2605.22800 emitted 0 images).
//! The probe must fall back to the project root.

use std::fs;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

#[test]
fn includegraphics_in_input_file_resolves_against_project_root() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    // <root>/figures/plot.png  — the real asset, root-relative.
    fs::create_dir_all(root.join("figures")).unwrap();
    fs::write(root.join("figures/plot.png"), b"\x89PNG\r\n").unwrap();
    // <root>/appendix/results.tex — an \input'd file referencing it root-relative.
    fs::create_dir_all(root.join("appendix")).unwrap();
    fs::write(
        root.join("appendix/results.tex"),
        "\\begin{figure}\n\\includegraphics{figures/plot.png}\n\\caption{Plot}\\label{fig:p}\n\\end{figure}\n",
    )
    .unwrap();
    let main = "Intro.\n\n\\input{appendix/results}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(root.to_path_buf()),
        },
    );
    let t = &out.typst;
    assert!(
        t.contains("image(\"figures/plot.png\")") || t.contains("image(\"figures/plot.png\","),
        "an \\input-ed figure's root-relative image must resolve, not be 'missing'; got:\n{t}"
    );
    assert!(
        !t.contains("(missing)"),
        "no missing-asset placeholder expected; got:\n{t}"
    );
    // The asset must be recorded so the project layer copies it.
    assert!(
        out.asset_refs.iter().any(|a| a.typst_path.contains("plot.png")),
        "image should be recorded as an asset ref; got refs: {:?}",
        out.asset_refs.iter().map(|a| &a.typst_path).collect::<Vec<_>>()
    );
}

#[test]
fn toplevel_figure_path_still_resolves() {
    // Regression guard: a figure in the MAIN file (base_dir == root) still works.
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    fs::write(root.join("solo.png"), b"\x89PNG\r\n").unwrap();
    let main = "\\begin{figure}\\includegraphics{solo.png}\\caption{Solo}\\end{figure}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(root.to_path_buf()),
        },
    );
    assert!(
        out.typst.contains("image(\"solo.png\")"),
        "top-level figure image still resolves; got:\n{}",
        out.typst
    );
}
