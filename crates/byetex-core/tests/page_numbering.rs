//! ByeTex emitted no page numbers, but every paper's truth PDF has them. Add
//! `numbering: "1"` to the document `#set page(...)`. Cover pages (thesis/report
//! `\coverimage`) suppress it. Found by the visual grader on 2605.22281 (SIAM).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn document_sets_page_numbering() {
    let t = typ(r"\documentclass{article}\begin{document}Body.\end{document}");
    // The page numbering (`"1"`) is distinct from heading numbering (`"1."`).
    assert!(t.contains("numbering: \"1\""), "no page numbering on #set page; got:\n{t}");
}

#[test]
fn bare_fragment_has_no_page_numbering() {
    let t = typ(r"Just a fragment.");
    assert!(!t.contains("numbering: \"1\""), "fragment should stay preamble-free; got:\n{t}");
}
