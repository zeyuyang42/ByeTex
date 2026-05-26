//! Snapshot tests for Phase 3 KaTeX single-arg wrapper coverage.
//! Verifies that newly-added wrap entries produce correct Typst math output
//! with zero conversion warnings.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn katex_phase3_accent_aliases() {
    // \widecheck is an alias for \check → caron(...)
    let src = r"$\widecheck{x} \check{y}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    let t = &out.typst;
    assert!(t.contains("caron(x)") || t.contains("caron("), "got: {t}");
}

#[test]
fn katex_phase3_mathring() {
    // \mathring → circle(...) ring accent
    let src = r"$\mathring{A}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("circle("), "got: {}", out.typst);
}

#[test]
fn katex_phase3_overrightarrow() {
    // \overrightarrow and \Overrightarrow → arrow(...)
    let src = r"$\overrightarrow{AB} \Overrightarrow{PQ}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("arrow("), "got: {}", out.typst);
}

#[test]
fn katex_phase3_font_aliases() {
    // \bold is an alias for \mathbf → bold(...)
    // \frak is an alias for \mathfrak → frak(...)
    let src = r"$\bold{x} \frak{y}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    let t = &out.typst;
    assert!(t.contains("bold("), "got: {t}");
    assert!(t.contains("frak("), "got: {t}");
}

#[test]
fn katex_phase3_phantom() {
    // \phantom → #hide[$y$] — `hide` is a content fn, needs # escape in math
    let src = r"$x + \phantom{y} + z$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("#hide["), "got: {}", out.typst);
}
