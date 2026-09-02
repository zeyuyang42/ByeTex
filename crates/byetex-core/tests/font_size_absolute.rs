//! LaTeX font-size declarations are ABSOLUTE, not relative.
//!
//! `\small` selects 9pt in a 10pt document no matter what is in force around it;
//! it does not scale the current size. Emitting `em` ratios made them compound:
//! a `\small` caption inside a `\footnotesize` table rendered at 0.9 x 0.8 =
//! 0.72em = 7.2pt, where LaTeX gives 9pt flat.
//!
//! Found on corpus 2606.12411, whose page 19 carries text at 4.5pt.

use byetex_core::{convert, ConvertOptions};

fn conv(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const TABLE: &str = "\\begin{tabular}{lc}\nA & 1 \\\\\n\\end{tabular}";

#[test]
fn a_float_size_switch_is_absolute() {
    let out = conv(&format!(
        "\\documentclass{{article}}\n\\begin{{document}}\n\
         \\begin{{table}}\n\\footnotesize\n{TABLE}\n\\end{{table}}\n\\end{{document}}"
    ));
    assert!(
        out.contains("text(size: 8pt)"),
        "\\footnotesize is 8pt in a 10pt document, not a ratio; got:\n{out}"
    );
}

#[test]
fn the_caption_rule_is_absolute() {
    let out = conv(
        "\\documentclass{article}\n\\captionsetup{font=small}\n\\begin{document}\n\
         \\begin{figure}\\caption{C}\\end{figure}\n\\end{document}",
    );
    assert!(
        out.contains("figure.caption: set text(size: 9pt)"),
        "\\small captions are 9pt, not 0.9em; got:\n{out}"
    );
}

#[test]
fn a_caption_inside_a_sized_float_does_not_compound() {
    // The case that motivated this: both rules apply to the same caption, and an
    // `em` pair multiplies into a size neither declaration asked for.
    let out = conv(&format!(
        "\\documentclass{{article}}\n\\captionsetup{{font=small}}\n\\begin{{document}}\n\
         \\begin{{table}}\n\\footnotesize\n\\caption{{C}}\n{TABLE}\n\\end{{table}}\n\\end{{document}}"
    ));
    assert!(
        !out.contains("0.72em") && !out.contains("em)["),
        "no em-relative sizing may survive where it can nest; got:\n{out}"
    );
    assert!(out.contains("text(size: 8pt)"), "float is 8pt; got:\n{out}");
    assert!(
        out.contains("figure.caption: set text(size: 9pt)"),
        "caption stays 9pt; got:\n{out}"
    );
}

#[test]
fn the_base_size_scales_the_declaration() {
    // An 11pt document: `\small` is not 9pt there. Ratios are applied to the
    // document's own base rather than assuming a 10pt class.
    let out = conv(&format!(
        "\\documentclass[11pt]{{article}}\n\\begin{{document}}\n\
         \\begin{{table}}\n\\small\n{TABLE}\n\\end{{table}}\n\\end{{document}}"
    ));
    assert!(
        out.contains("text(size: 9.9pt)"),
        "0.9 x 11pt = 9.9pt; got:\n{out}"
    );
}
