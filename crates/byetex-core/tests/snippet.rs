//! Tests for `byetex_core::snippet`: `explain` (per-node LaTeX→Typst mapping)
//! and `convert_fragment` (bare-fragment conversion with a context hint).

use byetex_core::snippet::{convert_fragment, explain, FragmentContext};
use byetex_core::ConvertOptions;

#[test]
fn explain_maps_fragments_back_to_source() {
    let src = "Hello \\textbf{world}.";
    let ex = explain(src, &ConvertOptions::default());
    assert!(!ex.is_empty(), "expected explanations; got {ex:?}");

    // Every explanation's byte range points back into the input and matches.
    for e in &ex {
        assert!(
            e.src_start < e.src_end && e.src_end <= src.len(),
            "bad range: {e:?}"
        );
        assert_eq!(
            &src[e.src_start..e.src_end],
            e.src_fragment,
            "range/fragment mismatch"
        );
    }

    // The bold command and its rendered text are represented somewhere.
    let all_src: String = ex.iter().map(|e| e.src_fragment.as_str()).collect();
    let all_typ: String = ex.iter().map(|e| e.typst_output.as_str()).collect();
    assert!(all_src.contains("textbf"), "src fragments: {all_src:?}");
    assert!(all_typ.contains("world"), "typst outputs: {all_typ:?}");
}

#[test]
fn convert_fragment_math_hint_produces_math() {
    // Bare `\frac{1}{2}` with a math hint must convert as math, not as an
    // unknown text command: `$\frac{1}{2}$` → `$(1) / (2)$`.
    let out = convert_fragment(
        "\\frac{1}{2}",
        FragmentContext::Math,
        &ConvertOptions::default(),
    );
    assert!(
        out.typst.contains('$'),
        "expected math wrap; got {:?}",
        out.typst
    );
    assert!(
        out.typst.contains('/'),
        "expected a fraction; got {:?}",
        out.typst
    );
}

#[test]
fn convert_fragment_inline_passes_through() {
    let out = convert_fragment(
        "\\textbf{hi}",
        FragmentContext::Inline,
        &ConvertOptions::default(),
    );
    assert!(
        out.typst.contains("*hi*") || out.typst.contains("strong"),
        "got {:?}",
        out.typst
    );
}

#[test]
fn fragment_context_parse_defaults_to_inline() {
    assert_eq!(FragmentContext::parse("math"), FragmentContext::Math);
    assert_eq!(
        FragmentContext::parse("math_display"),
        FragmentContext::MathDisplay
    );
    assert_eq!(FragmentContext::parse("block"), FragmentContext::Block);
    assert_eq!(FragmentContext::parse(""), FragmentContext::Inline);
    assert_eq!(FragmentContext::parse("nonsense"), FragmentContext::Inline);
}
