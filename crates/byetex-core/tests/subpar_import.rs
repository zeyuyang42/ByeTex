//! The `@preview/subpar` import is emitted exactly once, only when a
//! subpar.grid is present, and never for ordinary single-caption documents.
use byetex_core::convert;

#[test]
fn single_figure_document_has_no_subpar_import() {
    let src = "\\documentclass{article}\n\\begin{document}\n\
        \\begin{figure}\\includegraphics{x.png}\\caption{C}\\end{figure}\n\
        \\end{document}\n";
    let t = byetex_core::convert(src, &Default::default()).typst;
    assert!(
        !t.contains("@preview/subpar"),
        "single-caption doc must stay import-free; got:\n{t}"
    );
}
