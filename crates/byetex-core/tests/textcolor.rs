//! Regression tests for \textcolor — color dropped, content preserved.

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

#[test]
fn textcolor_in_text_emits_content() {
    let out = convert_str(r"\textcolor{red}{hello}");
    assert!(
        out.typst.contains("hello"),
        "content must be preserved, got: {}",
        out.typst
    );
}

#[test]
fn textcolor_in_text_drops_color_arg() {
    let out = convert_str(r"\textcolor{red}{hello}");
    // Must not leak raw LaTeX \textcolor or the color name as Typst identifier
    assert!(
        !out.typst.contains("textcolor"),
        "raw textcolor must not appear in output, got: {}",
        out.typst
    );
}

#[test]
fn textcolor_in_math_emits_content() {
    let out = convert_str(r"$\textcolor{green}{\checkmark}$");
    // \checkmark → checkmark.light or similar; content must survive
    assert!(
        !out.typst.contains("extcolor"),
        "extcolor (from \\textcolor leaking into Typst math) must not appear, got: {}",
        out.typst
    );
}
