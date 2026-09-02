//! LaTeX font-size DECLARATIONS inside a float (`\small`, `\footnotesize`, ...).
//!
//! These are switches, not commands with arguments: they apply from the point of
//! declaration to the end of the enclosing group/environment. The emitter dropped
//! every one of them, so a table an author had explicitly shrunk to fit rendered
//! at full body size. When the result no longer fits the column, Typst clamps the
//! overflow instead of breaking it: the excess rows are all painted at the same y,
//! producing an illegible pile-up. The text layer stays complete, so `word_recall`
//! scores it perfect and every existing gate is blind to it.
//!
//! Driver: corpus 2605.31604 (`\begin{table}[t]\centering\small ...`), whose
//! GenEval table piles 12 row labels on top of each other at the page bottom.

use byetex_core::{convert, ConvertOptions};

fn conv(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const TABLE: &str = "\\begin{tabular}{lc}\nA & 1 \\\\\nB & 2 \\\\\n\\end{tabular}";

#[test]
fn small_in_a_table_float_scales_the_table() {
    let out = conv(&format!(
        "\\begin{{table}}[t]\n\\centering\n\\small\n\\caption{{C}}\n{TABLE}\n\\end{{table}}"
    ));
    assert!(
        out.contains("text(size: 9pt)"),
        "\\small must scale the table; got:\n{out}"
    );
}

#[test]
fn footnotesize_and_scriptsize_use_their_own_ratios() {
    let foot = conv(&format!(
        "\\begin{{table}}\n\\footnotesize\n{TABLE}\n\\end{{table}}"
    ));
    assert!(foot.contains("text(size: 8pt)"), "footnotesize=0.8em; got:\n{foot}");
    let script = conv(&format!(
        "\\begin{{table}}\n\\scriptsize\n{TABLE}\n\\end{{table}}"
    ));
    assert!(script.contains("text(size: 7pt)"), "scriptsize=0.7em; got:\n{script}");
}

#[test]
fn the_last_declaration_wins() {
    // LaTeX switches are not nested here: the later one simply supersedes.
    let out = conv(&format!(
        "\\begin{{table}}\n\\scriptsize\n\\small\n{TABLE}\n\\end{{table}}"
    ));
    assert!(out.contains("text(size: 9pt)"), "later \\small wins; got:\n{out}");
    assert!(!out.contains("7pt"), "the superseded \\scriptsize must not survive; got:\n{out}");
}

#[test]
fn a_commented_out_declaration_is_ignored() {
    // Commenting the switch out is how an author disables it.
    let out = conv(&format!(
        "\\begin{{table}}\n%\\small\n{TABLE}\n\\end{{table}}"
    ));
    assert!(!out.contains("text(size:"), "a commented \\small must not fire; got:\n{out}");
}

#[test]
fn a_declaration_outside_the_float_does_not_leak_in() {
    // The switch belongs to the paragraph before the float, whose group ended.
    // Scanning back without a boundary would wrongly capture it.
    let out = conv(&format!(
        "\\small\n\nSome prose.\n\n\\begin{{table}}\n{TABLE}\n\\end{{table}}"
    ));
    assert!(
        !out.contains("text(size:"),
        "a switch outside the float must not wrap the table; got:\n{out}"
    );
}

#[test]
fn a_float_with_no_declaration_is_untouched() {
    // The control: no size switch, no wrapper, no change to existing output.
    let out = conv(&format!("\\begin{{table}}\n\\centering\n{TABLE}\n\\end{{table}}"));
    assert!(!out.contains("text(size:"), "must not wrap an unsized table; got:\n{out}");
}
