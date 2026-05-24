//! Regression test for the NBSP-in-math UTF-8 char-boundary panic.
//!
//! `collapse_math_spaces` used `s.rfind(|c| c.is_whitespace())` then
//! advanced the returned byte index by 1 to find the start of the
//! token *after* the whitespace. That works for ASCII whitespace
//! (which is 1 byte) but lands in the middle of any multi-byte
//! whitespace char — non-breaking space `\u{a0}` is 2 bytes in UTF-8
//! and is in Unicode's whitespace category, so `is_whitespace()`
//! matches it.
//!
//! Real-world trigger: arXiv source `2605.22765/appendix/audm.tex`
//! contains literal NBSP between math-mode tokens, which surfaced as
//! a hard panic at emit.rs:2420 once the run reached that file.

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
}

#[test]
fn nbsp_between_math_symbols_does_not_panic() {
    // `$\alpha<NBSP>\beta$` — two math symbols separated by a literal
    // non-breaking space. Each `\alpha`/`\beta` emission writes a
    // MATH_WORD_BOUNDARY sentinel, and `collapse_math_spaces` walks
    // those sentinels looking backwards for the previous token's
    // last whitespace.
    let nbsp = "\u{a0}";
    let src = format!(
        "\\documentclass{{article}}\\begin{{document}}$\\alpha{}\\beta$\\end{{document}}",
        nbsp
    );
    let out = convert_str(&src);
    // Just needs to not panic — both symbols should land in the
    // output as alpha/beta in some shape.
    assert!(
        out.typst.contains("alpha") && out.typst.contains("beta"),
        "expected alpha and beta in output; got:\n{}",
        out.typst
    );
}

#[test]
fn nbsp_between_text_words_in_math_does_not_panic() {
    // `$a<NBSP>b$` — purely textual, no symbol sentinel writes.
    // Still must not panic.
    let nbsp = "\u{a0}";
    let src = format!(
        "\\documentclass{{article}}\\begin{{document}}$a{}b$\\end{{document}}",
        nbsp
    );
    let _ = convert_str(&src);
}
