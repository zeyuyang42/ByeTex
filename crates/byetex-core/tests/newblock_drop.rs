//! `\newblock` is a bibliography entry-block separator (used inside
//! `\bibitem` / `thebibliography`). It carries no content and has no Typst
//! equivalent, so it should be dropped with NO warning — it was the single
//! largest `unsupported_command` source in the pinned corpus (~103 hits).

use byetex_core::{convert, Category, ConvertOptions};

fn out(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

#[test]
fn newblock_emits_nothing_and_does_not_warn() {
    let o = out("Smith, J.\\newblock A Great Paper.\\newblock Journal, 2020.");
    assert!(
        !o.typst.contains("newblock"),
        "\\newblock must not leak into output; got:\n{}",
        o.typst
    );
    let bad: Vec<_> = o
        .warnings
        .iter()
        .filter(|w| match &w.category {
            Category::UnsupportedCommand { name } | Category::DropOnly { name } => {
                name == "\\newblock"
            }
            _ => false,
        })
        .collect();
    assert!(
        bad.is_empty(),
        "\\newblock must not warn (neither unsupported nor drop_only); got: {bad:?}"
    );
}

#[test]
fn newblock_preserves_surrounding_reference_text() {
    let o = out("Smith, J.\\newblock A Great Paper.");
    assert!(
        o.typst.contains("Smith, J.") && o.typst.contains("A Great Paper."),
        "reference text on both sides of \\newblock must be preserved; got:\n{}",
        o.typst
    );
}
