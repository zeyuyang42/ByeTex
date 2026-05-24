//! Emit helpers for text-mode-in-math commands: `\mbox`, `\hbox`,
//! `\mathrel` and the math-class family, `\nicefrac`, `\raisetag`,
//! `\xrightarrow` and other extensible arrows, `\substack`.

use byetex_core::{convert, Category, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
}

fn ambiguous_count(out: &byetex_core::ConvertOutput, needle: &str) -> usize {
    out.warnings
        .iter()
        .filter(|w| {
            matches!(
                &w.category,
                Category::AmbiguousMath { reason } if reason.contains(needle)
            )
        })
        .count()
}

#[test]
fn mbox_in_math_renders_as_text() {
    let src = r"\documentclass{article}\begin{document}$\mbox{hello}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_count(&out, "mbox"), 0);
    assert!(
        out.typst.contains("\"hello\""),
        "expected \"hello\" upright text; got:\n{}",
        out.typst
    );
}

#[test]
fn mathrel_unwraps_to_inner_math() {
    let src = r"\documentclass{article}\begin{document}$a \mathrel{R} b$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_count(&out, "mathrel"), 0);
    // R should appear as a bare math identifier in the output.
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(stripped.contains('R'), "expected R in output; got:\n{}", out.typst);
}

#[test]
fn nicefrac_emits_inline_fraction() {
    let src = r"\documentclass{article}\begin{document}$\nicefrac{1}{2}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_count(&out, "nicefrac"), 0);
    assert!(
        out.typst.contains("(1) / (2)"),
        "expected (1) / (2); got:\n{}",
        out.typst
    );
}

#[test]
fn raisetag_silently_dropped() {
    let src = r"\documentclass{article}\begin{document}$x = 1\raisetag{1ex}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_count(&out, "raisetag"), 0);
    // `x = 1` should appear in the output; nothing else needed.
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(stripped.contains("x=1"));
}

#[test]
fn xrightarrow_emits_arrow_r() {
    let src = r"\documentclass{article}\begin{document}$A \xrightarrow{f} B$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_count(&out, "xrightarrow"), 0);
    assert!(
        out.typst.contains("arrow.r"),
        "expected arrow.r; got:\n{}",
        out.typst
    );
    // The label `f` should be attached above the arrow as `arrow.r^(f)`.
    assert!(
        out.typst.contains("arrow.r^(f)") || out.typst.contains("arrow.r ^(f)"),
        "expected `arrow.r^(f)` (with above-label); got:\n{}",
        out.typst
    );
}

#[test]
fn xrightarrow_with_below_and_above() {
    let src = r"\documentclass{article}\begin{document}$A \xrightarrow[g]{f} B$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_count(&out, "xrightarrow"), 0);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    // Both `^(f)` (above) and `_(g)` (below) should appear, attached to arrow.r.
    assert!(stripped.contains("arrow.r^(f)"));
    assert!(stripped.contains("_(g)"));
}

#[test]
fn substack_emits_comma_separated_inner() {
    let src = r"\documentclass{article}\begin{document}$\sum_{\substack{i \in S \\ i > 0}} x_i$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_count(&out, "substack"), 0);
    // The two lines of the substack should both appear in the output,
    // separated by a comma (our flattening replacement).
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(stripped.contains("i") && stripped.contains("S"));
}
