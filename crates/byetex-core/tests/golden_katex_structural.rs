/// Snapshot tests for Phase 4 KaTeX structural command coverage.
/// Verifies that newly-added structural arms produce correct Typst math output
/// with zero conversion warnings.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

// === Phase 4.1: frac variants ===

#[test]
fn katex_phase4_dfrac() {
    let src = r"$\dfrac{a}{b}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("/ ("), "got: {}", out.typst);
}

#[test]
fn katex_phase4_tfrac() {
    let src = r"$\tfrac{x}{y}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("/ ("), "got: {}", out.typst);
}

#[test]
fn katex_phase4_cfrac() {
    let src = r"$\cfrac{1}{n}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("/ ("), "got: {}", out.typst);
}

// === Phase 4.1: binom variants ===

#[test]
fn katex_phase4_dbinom() {
    let src = r"$\dbinom{n}{k}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("binom("), "got: {}", out.typst);
}

#[test]
fn katex_phase4_tbinom() {
    let src = r"$\tbinom{n}{k}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("binom("), "got: {}", out.typst);
}

// === Phase 4.2: horizontal braces ===

#[test]
fn katex_phase4_overbrace() {
    let src = r"$\overbrace{x + y}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("overbrace("), "got: {}", out.typst);
}

#[test]
fn katex_phase4_underbrace() {
    let src = r"$\underbrace{a + b}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("underbrace("), "got: {}", out.typst);
}

// === Phase 4.5: enclosures ===

#[test]
fn katex_phase4_cancel() {
    let src = r"$\cancel{x}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("cancel("), "got: {}", out.typst);
}

#[test]
fn katex_phase4_bcancel() {
    let src = r"$\bcancel{x}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    let t = &out.typst;
    assert!(t.contains("cancel(") && t.contains("inverted: true"), "got: {t}");
}

#[test]
fn katex_phase4_xcancel() {
    let src = r"$\xcancel{x}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    let t = &out.typst;
    assert!(t.contains("cancel(") && t.contains("cross: true"), "got: {t}");
}

#[test]
fn katex_phase4_sout() {
    let src = r"$\sout{x}$";
    let out = convert(src);
    assert!(out.warnings.is_empty(), "warnings: {:?}", out.warnings);
    assert!(out.typst.contains("strike("), "got: {}", out.typst);
}
