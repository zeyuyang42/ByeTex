//! Tests for citation-key validation (PR-3).
//!
//! When `\cite{key}` references a key that isn't defined by any
//! `.bib` / `.bbl` / `\bibitem` in the document, `emit_citation`
//! emits a plain-text placeholder instead of `@key`. Otherwise
//! Typst aborts the entire compile with `label <key> does not exist`.

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, Category, ConvertOptions};

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "byetex-cite-{}-{}",
        name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn cite_to_defined_key_emits_at_form() {
    let dir = tmpdir("defined");
    fs::write(
        dir.join("refs.bib"),
        "@article{Smith.2024, author={S}, year={2024}}\n",
    )
    .unwrap();
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\\begin{document}\
         See \\cite{Smith.2024}.\\bibliography{refs}\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(out.typst.contains("@Smith.2024"), "expected @Smith.2024; got:\n{}", out.typst);
    assert!(
        !out.typst.contains("missing key"),
        "defined key was flagged as missing; got:\n{}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cite_to_undefined_key_emits_placeholder() {
    let dir = tmpdir("undefined");
    // Only Smith is defined; cite to Jones should drop with placeholder.
    fs::write(
        dir.join("refs.bib"),
        "@article{Smith.2024, author={S}, year={2024}}\n",
    )
    .unwrap();
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\\begin{document}\
         See \\cite{Jones.2019}.\\bibliography{refs}\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(
        !out.typst.contains("@Jones.2019"),
        "undefined key emitted as @-ref; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("Jones.2019") && out.typst.contains("missing key"),
        "expected placeholder; got:\n{}",
        out.typst
    );
    // And there should be a NeedsManualReview warning naming the key.
    let has_warning = out.warnings.iter().any(|w| {
        matches!(&w.category, Category::NeedsManualReview { reason } if reason.contains("Jones.2019"))
    });
    assert!(has_warning, "no warning for undefined Jones.2019; got:\n{:?}", out.warnings);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cite_multi_key_partial_defined() {
    let dir = tmpdir("partial");
    fs::write(
        dir.join("refs.bib"),
        "@article{Smith.2024, year={2024}}\n",
    )
    .unwrap();
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\\begin{document}\
         See \\cite{Smith.2024,Jones.2019}.\\bibliography{refs}\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    // Defined key keeps @-form.
    assert!(out.typst.contains("@Smith.2024"));
    // Undefined key gets placeholder.
    assert!(out.typst.contains("Jones.2019") && out.typst.contains("missing key"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cite_with_no_bibliography_present_passes_through() {
    // Legacy convert call without any .bib / .bbl on disk should
    // skip validation and preserve the @-cite form (backwards
    // compat — the old behaviour with bare strings).
    let out = convert(
        "\\documentclass{article}\\begin{document}\
         See \\cite{Anywhere.2024}.\\end{document}\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            base_dir: None,
        },
    );
    assert!(
        out.typst.contains("@Anywhere.2024"),
        "no-base-dir cite should still emit @-form; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("missing key"),
        "no-base-dir mode should not flag missing; got:\n{}",
        out.typst
    );
}

#[test]
fn cite_to_bibitem_in_inlined_bbl_resolves() {
    // `.bbl` fallback's `\bibitem{key}` keys must register with the
    // validator so `\cite{key}` in the body keeps emitting @-form.
    let dir = tmpdir("bbl-cite");
    fs::write(
        dir.join("paper.bbl"),
        "\\begin{thebibliography}{99}\n\
         \\bibitem[S24]{Smith.2024}\nS. Author. Title. 2024.\n\
         \\end{thebibliography}\n",
    )
    .unwrap();
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\\begin{document}\
         See \\cite{Smith.2024}.\\bibliography{refs}\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(
        out.typst.contains("@Smith.2024"),
        "key from .bbl should validate as defined; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("missing key"),
        "key in .bbl wrongly flagged as missing; got:\n{}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}
