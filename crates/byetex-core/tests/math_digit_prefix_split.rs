/// Regression tests for digit-prefix math words (Bug #46-style).
/// LaTeX treats `2JX`, `2kg`, `2Np` in math as digit · letter · letter...
/// Typst reads them as identifiers → `unknown variable: JX` etc.
/// The fix: when a word node starts with digits followed by ≥2 alpha chars
/// that would normally be split, split the alpha portion as usual.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn digit_prefix_two_alpha_letters_are_split() {
    // LaTeX: $2JX$ — '2' is digit, 'JX' is multi-letter → should split 'J X'
    let src = r"$2JX$";
    let out = convert(src);
    assert!(
        !out.typst.contains("JX"),
        "JX must be split into individual letters, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("J X") || out.typst.contains("J  X"),
        "JX must appear as J <space> X in output, got: {}",
        out.typst
    );
}

#[test]
fn digit_prefix_kg_is_split() {
    // LaTeX: $2kg$ — common in physics papers (paper 22584 regression)
    let src = r"$2kg$";
    let out = convert(src);
    assert!(
        !out.typst.contains("kg"),
        "kg must be split into individual letters, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("k g") || out.typst.contains("k  g"),
        "kg must appear as k <space> g in output, got: {}",
        out.typst
    );
}

#[test]
fn digit_prefix_np_is_split() {
    // LaTeX: $2Np$ — paper 22795 regression (Np = neper unit)
    let src = r"$2Np$";
    let out = convert(src);
    assert!(
        !out.typst.contains("Np"),
        "Np must be split into individual letters, got: {}",
        out.typst
    );
}

#[test]
fn digit_prefix_single_letter_not_split() {
    // $2a$ — single letter after digit; single-char alpha doesn't need splitting
    let src = r"$2a$";
    let out = convert(src);
    // Just make sure '2' and 'a' both appear, no crash
    assert!(
        out.typst.contains('2') && out.typst.contains('a'),
        "both digit and letter must be in output, got: {}",
        out.typst
    );
}

#[test]
fn frac_braceless_arg_tail_letters_are_split() {
    // LaTeX: $\frac12Np_0$ — frac consumes '1' and '2' as braceless args,
    // leaving 'Np' as a partial-skip tail that must still be letter-split.
    // Paper 22795 regression.
    let src = r"$\frac12Np_0$";
    let out = convert(src);
    assert!(
        !out.typst.contains("Np"),
        "Np in frac tail must be split, got: {}",
        out.typst
    );
}

#[test]
fn plain_multi_letter_split_still_works() {
    // Regression: $ab$ (no digit prefix) should still split as before
    let src = r"$ab$";
    let out = convert(src);
    assert!(
        !out.typst.contains(" ab ") && (out.typst.contains("a b") || out.typst.contains("a  b")),
        "plain multi-letter word must still be split, got: {}",
        out.typst
    );
}

#[test]
fn hspace_after_letter_does_not_fuse_with_thin() {
    // LaTeX: $v\hspace{1em}$ — thin space must not fuse with preceding 'v'
    // producing 'vthin' (unknown variable in Typst). Paper 22728 regression.
    let src = r"$v\hspace{1em}$";
    let out = convert(src);
    assert!(
        !out.typst.contains("vthin"),
        "vthin must not appear; hspace must be separated from 'v', got: {}",
        out.typst
    );
}

#[test]
fn hspace_mid_math_does_not_fuse_identifiers() {
    // $v\hspace{-0.15em}\in$ — 'v' + thin + 'in' must stay separate
    let src = r"$v\hspace{-0.15em}\in$";
    let out = convert(src);
    assert!(
        !out.typst.contains("vthin"),
        "vthin must not appear, got: {}",
        out.typst
    );
}
