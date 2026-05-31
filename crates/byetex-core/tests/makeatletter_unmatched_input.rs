//! #4: an \input'ed fragment that opens \makeatletter without a matching
//! \makeatother (relying on the catcode persisting to EOF) must still harvest
//! its definitions, and must not leak the surrounding low-level TeX.

use std::fs;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

#[test]
fn unmatched_makeatletter_inputed_file_harvests_and_no_leak() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("helper.tex"),
        "\\makeatletter\n\\newcount\\rc@count\n\\rc@count=1\\relax\n\
         \\newcommand{\\helperval}{HVAL}\n",
    )
    .unwrap();
    let main = "\\documentclass{article}\n\\input{helper}\n\
                \\begin{document}\nResult: \\helperval.\n\\end{document}";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(tmp.path().to_path_buf()),
        },
    );
    assert!(
        out.typst.contains("HVAL"),
        "macro from unclosed-makeatletter include should still expand; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("=1"),
        "internals from the unclosed-makeatletter include leaked; got:\n{}",
        out.typst
    );
}
