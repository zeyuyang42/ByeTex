//! Phase 2c / defect D7: honor `\graphicspath{{dir1/}{dir2/}}`. LaTeX searches
//! those directories (relative to the main doc) for `\includegraphics{name}`,
//! so a bare `\includegraphics{plot.png}` whose file is at `figures/tasks/plot.png`
//! resolves via the graphicspath. ByeTex ignored `\graphicspath` and probed only
//! base_dir/root_dir directly, so such figures resolved as "missing" (corpus
//! 2605.22800 emitted 0 images for ~11 figures despite the files existing).

use std::fs;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

fn run(root: &std::path::Path, main: &str) -> byetex_core::ConvertOutput {
    convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(root.to_path_buf()),
        },
    )
}

#[test]
fn graphicspath_search_dir_resolves_bare_includegraphics() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    fs::create_dir_all(root.join("figures/tasks")).unwrap();
    fs::write(root.join("figures/tasks/plot.png"), b"\x89PNG\r\n").unwrap();
    let main = "\\graphicspath{{figures/main/}{figures/tasks/}}\n\
        \\begin{figure}\\includegraphics{plot.png}\\caption{P}\\end{figure}\n";
    let out = run(root, main);
    let t = &out.typst;
    assert!(
        t.contains("image(\"") && t.contains("plot.png"),
        "bare \\includegraphics must resolve via \\graphicspath search dir; got:\n{t}"
    );
    assert!(!t.contains("(missing)"), "no missing placeholder; got:\n{t}");
    assert!(
        out.asset_refs.iter().any(|a| a.source_path.ends_with("figures/tasks/plot.png")),
        "asset must point at the real file under the graphicspath dir; refs: {:?}",
        out.asset_refs.iter().map(|a| a.source_path.display().to_string()).collect::<Vec<_>>()
    );
}

#[test]
fn graphicspath_in_input_file_is_honored() {
    // \graphicspath often lives in preamble.tex, pulled in via \input.
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    fs::write(root.join("preamble.tex"), "\\graphicspath{{img/}}\n").unwrap();
    fs::create_dir_all(root.join("img")).unwrap();
    fs::write(root.join("img/fig.png"), b"\x89PNG\r\n").unwrap();
    let main = "\\input{preamble}\n\
        \\begin{figure}\\includegraphics{fig.png}\\caption{F}\\end{figure}\n";
    let out = run(root, main);
    assert!(
        out.typst.contains("fig.png") && !out.typst.contains("(missing)"),
        "graphicspath from an \\input-ed preamble must be honored; got:\n{}",
        out.typst
    );
}

#[test]
fn direct_path_still_resolves_without_graphicspath() {
    // Regression guard: no \graphicspath, direct relative path still works.
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    fs::write(root.join("solo.png"), b"\x89PNG\r\n").unwrap();
    let out = run(root, "\\begin{figure}\\includegraphics{solo.png}\\caption{S}\\end{figure}\n");
    assert!(out.typst.contains("image(\"solo.png\")"), "direct path still works; got:\n{}", out.typst);
}
