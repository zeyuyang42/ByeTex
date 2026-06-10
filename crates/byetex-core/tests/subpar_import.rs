//! The `@preview/subpar` import is emitted exactly once, only when a
//! subpar.grid is present, and never for ordinary single-caption documents.

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

#[test]
fn grid_document_imports_subpar_exactly_once() {
    let src = "\\documentclass{article}\n\\begin{document}\n\
        \\begin{figure}\n\
        \\begin{minipage}{0.5\\textwidth}\\includegraphics{a.png}\\captionof{figure}{L}\\label{f:a}\\end{minipage}\n\
        \\begin{minipage}{0.5\\textwidth}\\includegraphics{b.png}\\captionof{figure}{R}\\label{f:b}\\end{minipage}\n\
        \\end{figure}\n\\end{document}\n";
    let t = byetex_core::convert(src, &Default::default()).typst;
    assert_eq!(t.matches("@preview/subpar").count(), 1, "import exactly once; got:\n{t}");
    assert!(t.trim_start().starts_with("#import \"@preview/subpar"),
        "import must be at the very top; got:\n{}", &t[..t.len().min(200)]);
}
