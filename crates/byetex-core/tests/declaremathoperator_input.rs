//! Regression test: `\DeclareMathOperator` defined in an `\input`'d file must
//! be harvested, just like `\newcommand`.
//!
//! The project / `\input`-following macro harvester (`harvest_macros_from_source`)
//! handled `\newcommand`/`\def` but treated `\DeclareMathOperator` (also a
//! `new_command_definition` node) with the wrong extractor, so operators defined
//! in an included file (e.g. arXiv:2605.22159's `newcommands.tex`) were never
//! registered — every use emitted an `ambiguous_math` warning. With ~7 such
//! operators that paper produced ~365 warnings.

use std::fs;

use byetex_core::{convert, ConvertOptions};
use tempfile::TempDir;

fn convert_with_input(defs: &str, main_body: &str) -> byetex_core::ConvertOutput {
    let tmp = TempDir::new().expect("tempdir");
    fs::write(tmp.path().join("defs.tex"), defs).unwrap();
    let main = format!("\\input{{defs}}\n\\begin{{document}}\n{main_body}\n\\end{{document}}");
    convert(
        &main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(tmp.path().to_path_buf()),
        },
    )
}

fn has_ambiguous_math(out: &byetex_core::ConvertOutput, needle: &str) -> bool {
    out.warnings.iter().any(|w| {
        format!("{:?}", w.category).contains("AmbiguousMath") && {
            format!("{:?}", w.category).contains(needle) || w.message.contains(needle)
        }
    })
}

/// `\DeclareMathOperator` in an included file expands at the use site — it must
/// become an operator (`op(...)`), not an `ambiguous_math` placeholder.
#[test]
fn declaremathoperator_from_input_expands() {
    let out = convert_with_input(r"\DeclareMathOperator{\opL}{\mathcal L}", r"$\opL u$");
    assert!(
        out.typst.contains("op("),
        "\\opL should expand to an operator op(...);\noutput:\n{}",
        out.typst
    );
    assert!(
        !has_ambiguous_math(&out, "opL"),
        "\\opL must not warn ambiguous_math;\nwarnings:\n{:#?}",
        out.warnings
    );
}

/// The starred form `\DeclareMathOperator*` is also harvested from an include.
#[test]
fn declaremathoperator_star_from_input_expands() {
    let out = convert_with_input(
        r"\DeclareMathOperator*{\argmax}{arg\,max}",
        r"$\argmax_x f(x)$",
    );
    assert!(
        !has_ambiguous_math(&out, "argmax"),
        "\\argmax must not warn ambiguous_math;\nwarnings:\n{:#?}",
        out.warnings
    );
}

/// Sanity: a plain `\newcommand` from the same include still works (no
/// regression to the existing behavior this test relies on).
#[test]
fn newcommand_from_input_still_expands() {
    let out = convert_with_input(r"\newcommand{\foo}{BAR}", r"\foo");
    assert!(
        out.typst.contains("BAR"),
        "\\foo should expand to BAR;\noutput:\n{}",
        out.typst
    );
}
