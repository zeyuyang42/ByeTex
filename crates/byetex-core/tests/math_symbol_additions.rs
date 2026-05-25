//! Tests for symbol-table additions covering high-count residuals
//! across the arXiv corpus.

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

fn ambiguous_for(out: &byetex_core::ConvertOutput, needle: &str) -> usize {
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
fn vcentcolon_maps_to_colon() {
    // `\vcentcolon` (mathtools): vertically-centered colon. Drives ~135
    // warnings across the corpus.
    let src = r"\documentclass{article}\begin{document}$a\vcentcolon b$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "vcentcolon"), 0);
    let stripped: String = out.typst.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        stripped.contains("acolonb") || stripped.contains("a:b"),
        "expected `colon` between a and b; got:\n{}",
        out.typst
    );
}

#[test]
fn lbrace_rbrace_emit_braces() {
    // `\lbrace ... \rbrace` — alternate names for `\{...\}`. Drives ~100
    // warnings across the corpus.
    let src = r"\documentclass{article}\begin{document}$\lbrace x \rbrace$\end{document}";
    let out = convert_str(src);
    assert_eq!(
        ambiguous_for(&out, "lbrace") + ambiguous_for(&out, "rbrace"),
        0
    );
    assert!(
        out.typst.contains("\\{") && out.typst.contains("\\}"),
        "expected escaped braces in output; got:\n{}",
        out.typst
    );
}

#[test]
fn llbracket_rrbracket_emit_double_brackets() {
    // stmaryrd's `\llbracket ... \rrbracket` (Iverson-style). ~32 warnings.
    let src = r"\documentclass{article}\begin{document}$\llbracket x \rrbracket$\end{document}";
    let out = convert_str(src);
    assert_eq!(
        ambiguous_for(&out, "llbracket") + ambiguous_for(&out, "rrbracket"),
        0
    );
    assert!(
        out.typst.contains("bracket.l.double") && out.typst.contains("bracket.r.double"),
        "expected `bracket.l.double` and `bracket.r.double`; got:\n{}",
        out.typst
    );
}

#[test]
fn mathds_emits_bb_wrap() {
    // `\mathds{R}` (dsfont) — same visual as `\mathbb{R}`. ~122 warnings.
    let src = r"\documentclass{article}\begin{document}$\mathds{R}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "mathds"), 0);
    assert!(
        out.typst.contains("bb(R)"),
        "expected `bb(R)`; got:\n{}",
        out.typst
    );
}

#[test]
fn mathbbold_emits_bb_wrap() {
    // `\mathbbold{1}` (bbold) — blackboard-bold for digits.
    let src = r"\documentclass{article}\begin{document}$\mathbbold{1}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "mathbbold"), 0);
    assert!(
        out.typst.contains("bb(1)"),
        "expected `bb(1)`; got:\n{}",
        out.typst
    );
}

#[test]
fn forced_thin_space_emits_space() {
    // `\ ` (backslash-space) — LaTeX forced space in math. ~120 warnings.
    // Stress test: no panic, no warning.
    let src = "\\documentclass{article}\\begin{document}$a\\ b$\\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "\\ "), 0);
    // Result should have a/b separated by some whitespace.
    assert!(
        out.typst.contains("$a b$") || out.typst.contains("a b"),
        "expected `a b`; got:\n{}",
        out.typst
    );
}
