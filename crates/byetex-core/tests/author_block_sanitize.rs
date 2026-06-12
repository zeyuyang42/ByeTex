//! End-to-end: the rendered author block must be CLEAN — no raw LaTeX tokens —
//! and COMPLETE. Drives the full convert() path (the audit-leak fixtures).

use byetex_core::{convert, ConvertOptions};

fn render(class_and_author: &str) -> String {
    let src = format!(
        r"{class_and_author}\title{{T}}\begin{{document}}Body.\end{{document}}"
    );
    convert(&src, &ConvertOptions::default()).typst
}

/// The line(s) of the generated title block that carry author content: between
/// the title text and the abstract/keywords. We assert over the whole output
/// for simplicity since titles/sections here are trivial.
fn assert_clean(typst: &str) {
    for tok in ["\\,", "\\quad", "\\hspace", "\\thanks", "\\textbf", "\\\\", " & ", "\\}"] {
        assert!(
            !typst.contains(tok),
            "author block leaked `{tok}`:\n{typst}"
        );
    }
    // A leading comment percent must never appear at the start of an author line.
    assert!(!typst.contains("[% "), "leaked comment:\n{typst}");
    assert!(!typst.contains("% lead"), "leaked comment text:\n{typst}");
}

#[test]
fn neurips_comma_thinspace_block_is_clean() {
    // Mirrors 2605.22507: leading %, \, separators, trailing \}.
    let typst = render(
        "\\documentclass{article}\\usepackage{neurips_2026}\
         \\author{% lead\nPablo Moreno \\affiliation{UPF} \\email{p@upf.edu} \\and Adrian Müller \\affiliation{ETH}}",
    );
    assert_clean(&typst);
    assert!(typst.contains("Pablo Moreno"), "author 1 missing:\n{typst}");
    assert!(typst.contains("Adrian Müller"), "author 2 missing:\n{typst}");
}
