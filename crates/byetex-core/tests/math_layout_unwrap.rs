//! Tests for layout-primitive unwrap handlers in math mode.
//!
//! `\smash`, `\raisebox`, `\scalebox`, `\mathgroup` are LaTeX
//! positioning/styling primitives with no Typst equivalent for the
//! geometric arg, but their content should still render. ByeTex
//! previously dropped them with an ambiguous_math warning; this PR
//! emits only the content arg, dropping the positioning.

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
fn smash_unwraps_content() {
    // `\smash{X}` — emit only X. ~114 warnings in the corpus.
    let src = r"\documentclass{article}\begin{document}$\smash{X}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "smash"), 0);
    assert!(
        out.typst.contains("X"),
        "expected `X` in output; got:\n{}",
        out.typst
    );
}

#[test]
fn smash_with_optional_position_unwraps_content() {
    // `\smash[t]{X}` — optional `[t]` (or `[b]`) is dropped, content emitted.
    let src = r"\documentclass{article}\begin{document}$\smash[b]{X}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "smash"), 0);
    assert!(
        out.typst.contains("X") && !out.typst.contains("[b]"),
        "expected `X` without `[b]`; got:\n{}",
        out.typst
    );
}

#[test]
fn raisebox_unwraps_second_arg() {
    // `\raisebox{offset}{X}` — drop offset, emit X. ~504 warnings in
    // 2605.22765 alone.
    let src = r"\documentclass{article}\begin{document}$\raisebox{1pt}{X}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "raisebox"), 0);
    // Scope the check to the math span — the neutral preamble legitimately
    // contains `size: 11pt`, which would false-match a bare `contains("1pt")`.
    let math = out.typst.split('$').nth(1).unwrap_or("");
    assert!(
        math.contains("X") && !math.contains("1pt"),
        "expected `X` without offset `1pt`; got:\n{}",
        out.typst
    );
}

#[test]
fn scalebox_unwraps_second_arg() {
    // `\scalebox{factor}{X}` — drop factor, emit X.
    let src = r"\documentclass{article}\begin{document}$\scalebox{2.0}{X}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "scalebox"), 0);
    assert!(
        out.typst.contains("X") && !out.typst.contains("2.0"),
        "expected `X` without scaling factor; got:\n{}",
        out.typst
    );
}

#[test]
fn mathgroup_drops_group_code() {
    // `\mathgroup{N}{X}` — TeX font-group hint; emit content X.
    let src = r"\documentclass{article}\begin{document}$\mathgroup{0}{X}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "mathgroup"), 0);
}

#[test]
fn text_alias_textnormal_works() {
    // `\textnormal{X}` in math — same as `\text{X}`.
    let src = r"\documentclass{article}\begin{document}$\textnormal{hello}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "textnormal"), 0);
    assert!(
        out.typst.contains("\"hello\""),
        "expected `\"hello\"`; got:\n{}",
        out.typst
    );
}

#[test]
fn text_alias_texttt_works() {
    // `\texttt{X}` in math — same upright-quoted shape.
    let src = r"\documentclass{article}\begin{document}$\texttt{run}$\end{document}";
    let out = convert_str(src);
    assert_eq!(ambiguous_for(&out, "texttt"), 0);
    assert!(
        out.typst.contains("\"run\""),
        "expected `\"run\"`; got:\n{}",
        out.typst
    );
}

#[test]
fn text_with_ast_sibling_curly_does_not_warn() {
    // Reproduce the 2605.22584 leak: `\text{b}` inside a deeply
    // nested subscript where tree-sitter attaches `{b}` as an AST
    // sibling rather than child. The source-byte fallback should
    // catch it.
    let src = r"\documentclass{article}\begin{document}$\mathcal{V}_{\mathrm{n}_{\text{b}}}$\end{document}";
    let out = convert_str(src);
    assert_eq!(
        ambiguous_for(&out, "\\text"),
        0,
        "no \\text warning expected; got typst:\n{}",
        out.typst
    );
}
