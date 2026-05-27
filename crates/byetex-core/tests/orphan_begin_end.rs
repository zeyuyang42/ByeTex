use byetex_core::{convert, ConvertOptions};

fn no_begin_end_warning(src: &str) -> bool {
    let out = convert(src, &ConvertOptions::default());
    !out.warnings.iter().any(|w| {
        matches!(
            &w.category,
            byetex_core::warnings::Category::UnsupportedCommand { name }
            if name == "\\begin" || name == "\\end"
        )
    })
}

#[test]
fn orphan_end_document_no_warning() {
    // Snippet starts with \end{document} — tree-sitter sees it as a command.
    assert!(
        no_begin_end_warning(r"\end{document}"),
        "orphan \\end{{document}} should not warn"
    );
}

#[test]
fn orphan_begin_document_no_warning() {
    // Snippet that is just \begin{document} with no matching \end.
    assert!(
        no_begin_end_warning(r"\begin{document}Some text."),
        "orphan \\begin{{document}} should not warn"
    );
}

#[test]
fn orphan_end_tabular_no_warning() {
    assert!(
        no_begin_end_warning(r"\end{tabular}"),
        "orphan \\end{{tabular}} should not warn"
    );
}
