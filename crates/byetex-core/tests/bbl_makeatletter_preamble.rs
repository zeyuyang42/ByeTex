//! Regression (corpus 2605.31203): a BibTeX `.bbl` produced by apsrev4-1.bst
//! (natbib/revtex) opens its `thebibliography` with `\makeatletter` followed by
//! a block of `\providecommand \@...` internal-macro definitions, and **never**
//! emits a matching `\makeatother` — the at-letter catcode is simply expected to
//! persist to the end of the environment.
//!
//! ByeTex's `\makeatletter` region-skip (built for preamble internals) saw an
//! unmatched `\makeatletter` in a fragment with no `\documentclass` and skipped
//! to end-of-input, swallowing every `\bibitem`. The inlined `.bbl` then emitted
//! no `<key>` anchors, so each `\cite{...}` in the body dangled and Typst aborted
//! with "label `<key>` does not exist".
//!
//! The skip must stop at the first `\bibitem` / `\end{thebibliography}`: the
//! internal-macro preamble is still harvested and dropped, but the bibliography
//! entries (and their `<key>` anchors) render.

use byetex_core::{convert, ConvertOptions};

fn convert_inline(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            base_dir: None,
        },
    )
    .typst
}

#[test]
fn unmatched_makeatletter_in_thebibliography_keeps_bibitems() {
    // Minimal apsrev-style `.bbl`: `\makeatletter` + `\providecommand` internals
    // with NO `\makeatother`, then real bibitems.
    let src = "\\begin{thebibliography}{9}%\n\
        \\makeatletter\n\
        \\providecommand \\natexlab [1]{#1}%\n\
        \\providecommand \\bibinfo [2]{#2}%\n\
        \\bibitem [{Comaskey(2022)}]{comaskey:2022}\n\
        B.~Comaskey. \\bibinfo{journal}{Title}, 2022.\n\
        \\bibitem {bodo:2022}\n\
        A.~Bodo. Other, 2022.\n\
        \\end{thebibliography}\n";
    let out = convert_inline(src);
    assert!(
        out.contains("<comaskey:2022>"),
        "first bibitem anchor must survive the unmatched \\makeatletter; got:\n{}",
        out
    );
    assert!(
        out.contains("<bodo:2022>"),
        "second bibitem anchor must survive; got:\n{}",
        out
    );
    // The internal-macro preamble must NOT leak as visible text.
    assert!(
        !out.contains("@ifxundefined") && !out.contains("providecommand"),
        "internal-macro preamble must be skipped, not leaked; got:\n{}",
        out
    );
}
