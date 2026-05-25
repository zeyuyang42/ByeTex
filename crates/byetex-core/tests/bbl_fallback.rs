//! Tests for the `.bbl` rendering fallback (PR-2).
//!
//! When `\bibliography{Foo}` references a `.bib` that isn't bundled in
//! the source tree but a pre-rendered `.bbl` is present, the emitter
//! inlines the `.bbl` content via the existing `\bibitem` /
//! `thebibliography` path. Drives 2605.22159 (only ships
//! `GS4AGBEM.bbl`) and 2605.22776 (some `.bib` files missing).

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, Category, ConvertOptions};

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-bbl-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn bbl_inlined_when_bib_missing() {
    // arXiv pattern: `\bibliography{Refs}` but no `Refs.bib` on disk —
    // instead the author bundled a pre-rendered `Refs.bbl` (or any
    // `.bbl` matching the document stem).
    let dir = tmpdir("bbl-inline");
    fs::write(
        dir.join("paper.bbl"),
        "\\begin{thebibliography}{99}\n\
         \\bibitem[Smi24]{Smith:test}\n\
         A. Smith. \\emph{Title}. Journal, 2024.\n\
         \\end{thebibliography}\n",
    )
    .unwrap();
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\n\
         \\begin{document}\n\
         See \\cite{Smith:test}.\n\
         \\bibliography{Refs}\n\
         \\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    // Inlined bibitem should produce a `<Smith:test>` anchor matching
    // the `@Smith:test` cite in the body.
    assert!(
        out.typst.contains("@Smith:test"),
        "expected @Smith:test cite; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("<Smith:test>"),
        "expected <Smith:test> anchor from inlined .bbl; got:\n{}",
        out.typst
    );
    // The #bibliography call must NOT be emitted (no .bib file).
    assert!(
        !out.typst.contains("#bibliography"),
        "no #bibliography() should be emitted; got:\n{}",
        out.typst
    );
    // A NeedsManualReview info-level warning should explain the
    // fallback.
    let has_info = out.warnings.iter().any(|w| {
        matches!(
            &w.category,
            Category::NeedsManualReview { reason } if reason.contains("rendered `.bbl`")
        )
    });
    assert!(
        has_info,
        "expected .bbl-fallback warning; got:\n{:?}",
        out.warnings
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bibitem_with_optional_bracket_keeps_key() {
    // tree-sitter-latex parses `\bibitem[Agr02]{Agr:Foo}` with the
    // `[...]` and `{...}` as AST siblings — the original handler used
    // `first_curly_group` (children only) and missed the key entirely,
    // leaking `\bibitem[Agr02]{Agr:Foo}` as raw text. The source-byte
    // fallback now picks up the curly group.
    let out = convert(
        "\\begin{thebibliography}{99}\n\\bibitem[Agr02]{Agr:Foo}\nAuthor.\n\\end{thebibliography}\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            base_dir: None,
        },
    );
    assert!(
        out.typst.contains("<Agr:Foo>"),
        "expected <Agr:Foo> anchor; got:\n{}",
        out.typst
    );
}

#[test]
fn label_keys_sanitised_symmetrically() {
    // `\bibitem{DFG+:Foo}` defines a label; `\cite{DFG+:Foo}` in body
    // references it. Typst rejects `+` in label keys, so both sides
    // are sanitized to `DFG-:Foo` — they still match.
    let out = convert(
        "See \\cite{DFG+:Foo}.\n\
         \\begin{thebibliography}{99}\n\\bibitem{DFG+:Foo}Author.\n\\end{thebibliography}\n",
        &ConvertOptions {
            source_name: Some("inline".into()),
            base_dir: None,
        },
    );
    // Both occurrences should be sanitized to use `-` instead of `+`.
    assert!(
        out.typst.contains("@DFG-:Foo"),
        "expected @DFG-:Foo (sanitized cite); got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("<DFG-:Foo>"),
        "expected <DFG-:Foo> (sanitized label); got:\n{}",
        out.typst
    );
    // Original key with `+` must NOT appear (would crash Typst).
    assert!(
        !out.typst.contains("<DFG+:Foo>"),
        "unsanitized label key leaked; got:\n{}",
        out.typst
    );
}
