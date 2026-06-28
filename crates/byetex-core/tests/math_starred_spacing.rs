//! Starred math spacing/tag primitives (`\hspace*`, `\vspace*`, `\tag*`) must
//! behave like their non-starred forms instead of leaking the bare command name
//! as an `ambiguous_math` string. Non-starred `\hspace`→`thin`, `\vspace`/`\tag`
//! are dropped; before the fix the starred variants fell through to
//! `emit_unknown_math_command` and rendered as `"hspace*"` / `"tag*"` (dogfood
//! 2605.22728: `\hspace*` in math leaked as the literal text `hspace*`).

use byetex_core::{convert, Category, ConvertOptions};

fn out(src: &str) -> byetex_core::ConvertOutput {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
}

fn ambiguous_reasons(o: &byetex_core::ConvertOutput) -> Vec<String> {
    o.warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::AmbiguousMath { reason } => Some(reason.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn hspace_star_in_math_does_not_leak() {
    let o = out(r"\documentclass{article}\begin{document}$x \hspace*{2mm} y$\end{document}");
    assert!(
        !o.typst.contains("hspace"),
        "`\\hspace*` leaked into math:\n{}",
        o.typst
    );
    assert!(
        ambiguous_reasons(&o).iter().all(|r| !r.contains("hspace")),
        "ambiguous_math for \\hspace*: {:?}",
        ambiguous_reasons(&o)
    );
    // Matches the non-starred behaviour: a thin space.
    assert!(o.typst.contains("thin"), "expected `thin`:\n{}", o.typst);
}

#[test]
fn tag_star_in_math_does_not_leak() {
    let o = out(r"\documentclass{article}\begin{document}\begin{equation}x=1\tag*{(A)}\end{equation}\end{document}");
    assert!(
        !o.typst.contains("tag*") && !o.typst.contains("\"tag"),
        "`\\tag*` leaked into math:\n{}",
        o.typst
    );
    assert!(
        ambiguous_reasons(&o).iter().all(|r| !r.contains("tag")),
        "ambiguous_math for \\tag*: {:?}",
        ambiguous_reasons(&o)
    );
}

#[test]
fn vspace_star_in_math_does_not_leak() {
    let o = out(r"\documentclass{article}\begin{document}$\vspace*{1mm} z$\end{document}");
    assert!(
        !o.typst.contains("vspace"),
        "`\\vspace*` leaked into math:\n{}",
        o.typst
    );
}
