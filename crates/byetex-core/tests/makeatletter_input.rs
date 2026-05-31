//! #1 regression: in single-file (non-project) `\input` mode, a definition
//! inside a `\makeatletter ... \makeatother` block of an `\input`'ed file must
//! still be harvested. The child emitter that expands `\input` runs no prepass
//! (it relies on emit-time harvesting during the walk); the region-skip would
//! otherwise drop those definitions, so the harvest must happen at the skip.

use std::fs;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

fn run_with_base(main: &str, base: std::path::PathBuf) -> byetex_core::ConvertOutput {
    convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(base),
        },
    )
}

#[test]
fn newcommand_in_makeatletter_of_inputed_file_still_expands() {
    let tmp = TempDir::new().expect("tempdir");
    // Mirrors corpus/2605.22159: \input{newcommands} where the file defines a
    // macro inside a \makeatletter region.
    fs::write(
        tmp.path().join("newcommands.tex"),
        "\\makeatletter\n\\newcommand{\\myconst}{EXPANDEDVALUE}\n\\makeatother\n",
    )
    .unwrap();
    let main = "\\documentclass{article}\n\\input{newcommands}\n\
                \\begin{document}\nValue is \\myconst.\n\\end{document}";
    let out = run_with_base(main, tmp.path().to_path_buf());
    assert!(
        out.typst.contains("EXPANDEDVALUE"),
        "macro defined in a \\makeatletter region of an \\input'ed file should still expand; got:\n{}",
        out.typst
    );
}

#[test]
fn def_in_makeatletter_of_inputed_file_still_expands() {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("defs.tex"),
        "\\makeatletter\n\\def\\foozle{ZLEVALUE}\n\\makeatother\n",
    )
    .unwrap();
    let main = "\\documentclass{article}\n\\input{defs}\n\
                \\begin{document}\nGot \\foozle.\n\\end{document}";
    let out = run_with_base(main, tmp.path().to_path_buf());
    assert!(
        out.typst.contains("ZLEVALUE"),
        "\\def macro inside a \\makeatletter region of an \\input'ed file should still expand; got:\n{}",
        out.typst
    );
}
