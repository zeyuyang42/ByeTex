//! biblatex `\printbibliography` renders `#bibliography(...)` from
//! `\addbibresource` paths collected in the prepass — but `\addbibresource` is
//! often declared in a *class/config* file the prepass doesn't see (e.g.
//! `internshipreport.cls` / `config/packages.tex`), so the path list is empty,
//! the bibliography is dropped, and every `\cite` dangles → the whole Typst
//! compile hard-fails. When no resource path is known, fall back to discovering
//! `.bib` files in the project tree (incl. subdirs) and render from those.
//! Found via the cross-doc-type render gallery (gh-sikatikenmogne-report).

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-pb-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn printbibliography_falls_back_to_discovered_subdir_bib() {
    let dir = tmpdir("printbib-subdir");
    fs::create_dir_all(dir.join("content/backmatter")).unwrap();
    fs::write(
        dir.join("content/backmatter/bibliography.bib"),
        "@book{cohn2009succeeding,\n  author = {Cohn, M.},\n  title = {Succeeding},\n  year = {2009}\n}\n",
    )
    .unwrap();
    // No `\addbibresource` in the main file (it lives in the class/config) and
    // no top-level `.bib` — only the subdir one.
    fs::write(
        dir.join("main.tex"),
        "\\documentclass{article}\n\
         \\begin{document}\n\
         Body \\cite{cohn2009succeeding}.\n\
         \\printbibliography\n\
         \\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("main.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("main.tex")).unwrap(), &opts);

    assert!(
        out.typst.contains("#bibliography("),
        "\\printbibliography did not render from the discovered subdir .bib; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("content/backmatter/bibliography.bib"),
        "discovered subdir .bib path not used; got:\n{}",
        out.typst
    );
    // The cited key must still be referenced (it now resolves against the
    // rendered #bibliography rather than dangling).
    assert!(
        out.typst.contains("@cohn2009succeeding"),
        "cite lost; got:\n{}",
        out.typst
    );
}

#[test]
fn printbibliography_still_dropped_when_no_bib_anywhere() {
    let dir = tmpdir("printbib-none");
    fs::write(
        dir.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\nBody.\n\\printbibliography\n\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("main.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("main.tex")).unwrap(), &opts);
    assert!(
        !out.typst.contains("#bibliography("),
        "rendered a bibliography with no .bib present; got:\n{}",
        out.typst
    );
}
