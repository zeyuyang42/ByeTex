//! A `\cite{key}` whose key is NOT in the bibliography made the WHOLE Typst
//! document fail to compile (`label <key> does not exist`), where pdflatex would
//! render `[?]`. ByeTex already degrades undefined cites to a text placeholder —
//! but only when `bibliography_keys` is non-empty, and the prepass only harvested
//! `.bib`/`.bbl` files, not an inline `\begin{thebibliography}`. So with an inline
//! bibliography (no `.bib`), every cite was emitted as `@key` and a single
//! dangling one sank the compile. The prepass now harvests inline `\bibitem` keys.
//! Found via the cross-doc-type render gallery (gh-sikatikenmogne-report).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const SRC: &str = r"\documentclass{article}
\begin{document}
Hello \cite{realkey} and \cite{missingkey}.
\begin{thebibliography}{9}
\bibitem{realkey} A. Author, A real entry, 2020.
\end{thebibliography}
\end{document}";

#[test]
fn defined_cite_resolves_undefined_does_not_emit_dangling_ref() {
    let t = typ(SRC);
    // The defined key still becomes a real reference…
    assert!(t.contains("@realkey"), "defined cite lost; got:\n{t}");
    // …but the undefined key must NOT be emitted as a dangling `@missingkey`
    // (which would make typst hard-fail "label <missingkey> does not exist").
    assert!(
        !t.contains("@missingkey"),
        "undefined cite emitted a dangling reference; got:\n{t}"
    );
    // It degrades to a visible placeholder instead of vanishing silently.
    assert!(
        t.contains("missing key") && t.contains("missingkey"),
        "undefined cite should leave a placeholder; got:\n{t}"
    );
}
