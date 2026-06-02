//! Expanded-corpus compile-blocker (2605.31443): a `\bibliography{...}` whose
//! path list spans multiple lines —
//!     \bibliography{
//!     bib/references,
//!     bib/general
//!     }
//! extracted path tokens carrying leading/trailing newlines and spaces
//! (`"\nbib/references"`), which then failed `probe_bib_on_disk` → the whole
//! `#bibliography(...)` call was dropped → every `\cite{key}` dangled
//! (`label <rct> does not exist`). Each extracted path must be trimmed.

use std::fs;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

#[test]
fn multiline_bibliography_paths_resolve() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    fs::create_dir_all(root.join("bib")).unwrap();
    fs::write(
        root.join("bib/general.bib"),
        "@book{rct,\n  title={T}, author={A}, year={1955}\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("bib/references.bib"),
        "@article{foo,\n  title={T}, author={A}, year={2020}\n}\n",
    )
    .unwrap();
    let main = "\\documentclass{article}\\begin{document}\n\
        See \\cite{rct}.\n\
        \\bibliography{\nbib/references,\nbib/general\n% bib/commented_out,\n}\n\\end{document}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(root.to_path_buf()),
        },
    );
    let t = &out.typst;
    assert!(
        t.contains("#bibliography("),
        "a multi-line \\bibliography must still emit #bibliography(); got:\n{t}"
    );
    assert!(
        t.contains("\"bib/references.bib\"") && t.contains("\"bib/general.bib\""),
        "both trimmed paths must resolve; got:\n{t}"
    );
}
