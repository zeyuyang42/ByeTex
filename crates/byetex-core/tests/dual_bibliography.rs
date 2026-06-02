//! Expanded-corpus compile-blocker (2605.31440): the source has BOTH a
//! `\bibliography{paper-ref}` (→ `#bibliography("paper-ref.bib")`) AND a manual
//! `\begin{thebibliography}` whose `\bibitem{key}` entries byetex emits as
//! `#figure(kind: "bibitem") <key>`. The same key then lives in two places —
//! the document (`<key>` label) and the bibliography (.bib entry) — and Typst
//! aborts: `label <key> occurs both in the document and its bibliography`.
//!
//! A document has ONE bibliography. When a `\bibliography{refs}` resolves to a
//! real `.bib`, its `#bibliography(.bib)` is the complete, canonical reference
//! list (the paper cites many keys that live ONLY in the .bib). The manual
//! `thebibliography` is then redundant and must be dropped — keeping it would
//! re-declare the shared keys as document labels and collide with the .bib.

use std::fs;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

#[test]
fn resolvable_bib_drops_manual_thebibliography() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    fs::write(
        root.join("refs.bib"),
        "@article{andersen1955,\n  title={X},\n  author={A},\n  year={1955}\n}\n",
    )
    .unwrap();
    let main = "\\documentclass{article}\n\\begin{document}\n\
        See \\cite{andersen1955}.\n\
        \\bibliography{refs}\n\
        \\begin{thebibliography}{9}\n\
        \\bibitem{andersen1955} Andersen, 1955.\n\
        \\end{thebibliography}\n\\end{document}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(root.to_path_buf()),
        },
    );
    let t = &out.typst;
    // The .bib reference list is the authoritative one.
    assert!(
        t.contains("#bibliography("),
        "a resolvable \\bibliography{{refs}} must emit #bibliography(); got:\n{t}"
    );
    // The redundant manual bibitem (which would declare `<andersen1955>` and
    // collide with the .bib) must be dropped.
    assert!(
        !t.contains("kind: \"bibitem\""),
        "manual \\bibitem must be dropped when the .bib is authoritative; got:\n{t}"
    );
    assert!(
        !t.contains("<andersen1955>"),
        "no document label may shadow the .bib key; got:\n{t}"
    );
}

#[test]
fn manual_thebibliography_kept_without_bib_file() {
    // No .bib on disk → the manual list IS the bibliography; keep it.
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let main = "\\documentclass{article}\n\\begin{document}\n\
        See \\cite{smith2020}.\n\
        \\begin{thebibliography}{9}\n\
        \\bibitem{smith2020} Smith, 2020.\n\
        \\end{thebibliography}\n\\end{document}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(root.to_path_buf()),
        },
    );
    assert!(
        out.typst.contains("kind: \"bibitem\""),
        "with no .bib, the manual \\bibitem must render; got:\n{}",
        out.typst
    );
}

#[test]
fn bibliography_call_kept_without_manual_thebibliography() {
    // Regression guard: with NO thebibliography, `\bibliography{refs}` must
    // still emit `#bibliography(...)`.
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    fs::write(
        root.join("refs.bib"),
        "@article{andersen1955,\n  title={X},\n  author={A},\n  year={1955}\n}\n",
    )
    .unwrap();
    let main = "\\documentclass{article}\n\\begin{document}\n\
        See \\cite{andersen1955}.\n\\bibliography{refs}\n\\end{document}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(root.to_path_buf()),
        },
    );
    assert!(
        out.typst.contains("#bibliography("),
        "\\bibliography without a manual list must still emit #bibliography(); got:\n{}",
        out.typst
    );
}
