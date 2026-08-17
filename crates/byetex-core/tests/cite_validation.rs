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
    let dir = std::env::temp_dir().join(format!("byetex-cite-{}-{}", name, std::process::id()));
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
    assert!(
        out.typst.contains("@Smith.2024"),
        "expected @Smith.2024; got:\n{}",
        out.typst
    );
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
    assert!(
        has_warning,
        "no warning for undefined Jones.2019; got:\n{:?}",
        out.warnings
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cite_multi_key_partial_defined() {
    let dir = tmpdir("partial");
    fs::write(dir.join("refs.bib"), "@article{Smith.2024, year={2024}}\n").unwrap();
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

#[test]
fn unicode_cite_key_is_preserved_not_sanitized() {
    // Bib keys with non-ASCII letters (e.g. `HintermüllerKunisch2004`) must
    // NOT be mangled to `Hinterm-llerKunisch2004`. Typst labels support
    // Unicode. Paper 22728 regression.
    let src = r"\begin{document}
\cite{HintermüllerKunisch2004}
\begin{thebibliography}{99}
\bibitem{HintermüllerKunisch2004} Hintermuller et al.
\end{thebibliography}
\end{document}";
    let out = byetex_core::convert(src, &Default::default());
    assert!(
        out.typst.contains("@HintermüllerKunisch2004"),
        "unicode cite key must be preserved, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("Hinterm-llerKunisch2004"),
        "mangled key must not appear, got: {}",
        out.typst
    );
}

// ── a key that EXISTS but will never be DEFINED in the output ───────────────
//
// The validation above asks "is this key in a `.bib` on disk?" when the question
// that decides whether Typst compiles is "will this key be DEFINED in the
// document we emit?". Those come apart whenever the bibliography command is one
// ByeTex does not support: `refs.bib` harvests fine, so `\cite{k}` validates and
// emits `@k` — but no `#bibliography()` is ever written, so nothing defines
// `<k>` and Typst aborts the WHOLE compile.
//
// Real corpus case: `gh-maurovm-thesis-template` uses the oxengthesis class's
// `\listofreferences`, an unsupported command. `references.bib` sits right there
// in the source dir, so every `\cite` passed validation and the conversion has
// never compiled:
//     error: label `<prior1977physical>` does not exist in the document

#[test]
fn cite_without_a_rendering_bibliography_degrades_to_placeholder() {
    let dir = tmpdir("nobib-render");
    fs::write(
        dir.join("references.bib"),
        "@book{prior1977physical, author={Prior}, year={1977}}\n",
    )
    .unwrap();
    // `\listofreferences` stands in for any bibliography command ByeTex does not
    // support: the key resolves on disk, but nothing emits an anchor for it.
    fs::write(
        dir.join("thesis.tex"),
        "\\documentclass{article}\\begin{document}\
         Vital signs \\cite{prior1977physical}.\\listofreferences\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("thesis.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("thesis.tex")).unwrap(), &opts);
    assert!(
        !out.typst.contains("@prior1977physical"),
        "emitted a dangling `@key` with no `#bibliography` to define it — this \
         aborts the entire Typst compile; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("prior1977physical"),
        "the key should survive as readable text, not vanish; got:\n{}",
        out.typst
    );
    assert!(
        out.warnings
            .iter()
            .any(|w| matches!(&w.category, Category::NeedsManualReview { .. })),
        "a dropped citation must be warned about, not silently degraded"
    );
    let _ = fs::remove_dir_all(&dir);
}

// ── the bibliography is REACHED, just not from the top-level prepass ────────
//
// `bib_will_render` is set only by `prepass_collect`, which walks the top-level
// tree. But the emit pass resolves `\input`, and `\printbibliography` falls back
// to `discover_bib_files()` when no `\addbibresource` path was collected. So a
// real `#bibliography(...)` renders while the flag stays false — and then EVERY
// citation degrades to `[cite: key]` next to a fully-rendered reference list.
//
// That is why the decision cannot be made when the `\cite` is emitted: the
// citation comes first in document order, the bibliography command later. It has
// to be deferred to `finish()`, where `emitted_bibliography` is authoritative.

#[test]
fn addbibresource_in_an_included_file_still_resolves_cites() {
    let dir = tmpdir("input-addbibresource");
    fs::write(
        dir.join("refs.bib"),
        "@book{prior1977physical, author={Prior}, year={1977}}\n",
    )
    .unwrap();
    // The resource declaration lives in an `\input`ed file, so the top-level
    // prepass never sees the `biblatex_include` node.
    fs::write(dir.join("setup.tex"), "\\addbibresource{refs.bib}\n").unwrap();
    fs::write(
        dir.join("thesis.tex"),
        "\\documentclass{article}\\input{setup}\\begin{document}\
         Vital signs \\cite{prior1977physical}.\\printbibliography\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("thesis.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("thesis.tex")).unwrap(), &opts);
    assert!(
        out.typst.contains("#bibliography("),
        "precondition: a real bibliography must render here; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("@prior1977physical"),
        "a rendered `#bibliography(...)` DEFINES the key, so `@key` must be kept — \
         degrading to a plain-text placeholder next to a rendered reference list \
         loses every citation in the document; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("[cite: prior1977physical]"),
        "the placeholder must not appear when the bibliography renders; got:\n{}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bibliography_command_in_an_included_file_still_resolves_cites() {
    let dir = tmpdir("input-bibliography");
    fs::write(
        dir.join("refs.bib"),
        "@book{prior1977physical, author={Prior}, year={1977}}\n",
    )
    .unwrap();
    // Back matter in its own file — the `bibtex_include` node is invisible to
    // the top-level prepass, so `bib_will_render` stays false.
    fs::write(dir.join("backmatter.tex"), "\\bibliography{refs}\n").unwrap();
    fs::write(
        dir.join("thesis.tex"),
        "\\documentclass{article}\\begin{document}\
         Vital signs \\cite{prior1977physical}.\\input{backmatter}\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("thesis.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("thesis.tex")).unwrap(), &opts);
    assert!(
        out.typst.contains("#bibliography("),
        "precondition: a real bibliography must render here; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("@prior1977physical"),
        "an `\\input`ed `\\bibliography{{}}` still defines the key; got:\n{}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn no_deferred_cite_sentinel_survives_into_the_output() {
    // The deferral resolves inside `finish()`. If any path ever bypassed that
    // resolution the sentinel would ship as literal garbage in the .typ, so
    // assert on the control character directly rather than on either outcome.
    let dir = tmpdir("sentinel-leak");
    fs::write(
        dir.join("refs.bib"),
        "@book{k1, author={A}, year={1999}}\n@book{k2, author={B}, year={2000}}\n",
    )
    .unwrap();
    fs::write(dir.join("setup.tex"), "\\addbibresource{refs.bib}\n").unwrap();
    fs::write(
        dir.join("main.tex"),
        "\\documentclass{article}\\input{setup}\\begin{document}\
         A \\cite{k1} and B \\cite{k1,k2}.\\printbibliography\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("main.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("main.tex")).unwrap(), &opts);
    // `DEFERRED_CITE_SENTINEL` (crate-private); the neighbouring markers are
    // \u{1d} cell-keep, \u{1e} box, \u{1f} ref-key — none may ship either.
    assert!(
        !out.typst.contains(|c| ('\u{1c}'..='\u{1f}').contains(&c)),
        "an emitter sentinel leaked into the emitted Typst:\n{:?}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn deferred_cite_at_the_end_of_emphasis_keeps_the_function_form() {
    // `_… @key_` is invalid Typst: the closing `_` is absorbed into the label
    // name, so the emphasis never closes (`unclosed delimiter`). The emitter
    // guards this by falling back to `#emph[…]` when the content ends in a
    // reference — a guard that reads the ALREADY-EMITTED text. Deferring the
    // citation must therefore not hide the `@key` from it.
    //
    // Caught on gh-dzwaneveld-tudelft-thesis: an earlier sentinel that WRAPPED
    // the token flipped this back to `_… @example-article_` and the paper stopped
    // compiling — while the acceptance gate stayed green, because it filed the
    // failure as INPUT_BROKEN.
    let dir = tmpdir("emph-tail");
    fs::write(
        dir.join("refs.bib"),
        "@article{example-article, author={A}, year={2024}}\n",
    )
    .unwrap();
    // No bibliography command at all, so the cite is deferred and then degrades.
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\\begin{document}\
         \\textit{An introduction \\cite{example-article}}\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(
        !out.typst.contains("_An introduction"),
        "emphasis ending in a (deferred) reference must use `#emph[…]`, not `_…_`; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("#emph["),
        "expected the function form; got:\n{}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cite_with_thebibliography_still_emits_at_form() {
    // The control that stops the fix over-reaching: no `#bibliography()` renders
    // here either, but `\bibitem` DOES emit a `<key>` anchor, so `@key` resolves
    // and must be preserved.
    let dir = tmpdir("thebib-anchor");
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\\begin{document}\
         See \\cite{Knuth1984}.\
         \\begin{thebibliography}{9}\\bibitem{Knuth1984} D. Knuth.\\end{thebibliography}\
         \\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(
        out.typst.contains("@Knuth1984"),
        "a \\bibitem anchor DOES define the key — `@key` must be kept; got:\n{}",
        out.typst
    );
    let _ = fs::remove_dir_all(&dir);
}
