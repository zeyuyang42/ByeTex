//! A leaked `\begin{document}` / `\end{document}` must never appear in the
//! output. These markers only ever reach the body when tree-sitter fails to form
//! the `document` environment (ERROR recovery, e.g. an unclosed environment
//! elsewhere) and the emitter raw-copies the loose `begin`/`end` node — the
//! well-formed case consumes them as the document delimiters. They have no Typst
//! rendering, so they're dropped (L1 sub-fix on corpus 2605.22728).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
    .typst
}

#[test]
fn leaked_document_markers_are_dropped() {
    // The unclosed `\begin{align}` makes tree-sitter fail to form the document
    // environment, so `\begin{document}`/`\end{document}` end up as loose nodes.
    let src = r"\documentclass{article}
\begin{document}
\begin{align} x=1
\section{S}
Body.
\end{document}";
    let t = typ(src);
    // The loose `\begin{document}` node must be dropped, not raw-copied. (Whether
    // the trailing `\end{document}` stays a loose node or is consumed as a
    // mismatched-environment delimiter depends on the exact malformation; the
    // emitter drops it in BOTH the `begin` and `end` loose-node forms — verified
    // end-to-end on corpus 2605.22728, where both markers are loose and both go.)
    assert!(
        !t.contains(r"\begin{document}"),
        "leaked `\\begin{{document}}` in output:\n{t}"
    );
}

#[test]
fn verbatim_begin_document_is_preserved() {
    // A `\begin{document}` shown inside a verbatim/listing is real content (a code
    // listing) and must NOT be dropped — it's a string token, not a `begin` node.
    let src = r"\documentclass{article}\begin{document}
\begin{verbatim}
\begin{document}
\end{verbatim}
\end{document}";
    let t = typ(src);
    assert!(
        t.contains("begin{document}"),
        "verbatim code listing lost its `\\begin{{document}}`:\n{t}"
    );
}
