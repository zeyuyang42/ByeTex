//! array-package `>{…}` column decorators used to be parsed and DISCARDED, so a
//! `>{\centering\arraybackslash}p{3cm}` column silently lost its alignment
//! override (corpus: 18 papers). The decorator's alignment now propagates to the
//! following column's Typst `align`. Non-alignment decorators (`\bfseries`, …) are
//! still dropped — only the column alignment is recovered.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn wrap_tabular(spec: &str) -> String {
    format!(
        "\\documentclass{{article}}\\begin{{document}}\\begin{{tabular}}{{{spec}}}\nA & B \\\\\n\\end{{tabular}}\\end{{document}}\n"
    )
}

#[test]
fn centering_decorator_overrides_p_column_to_center() {
    let t = typ(&wrap_tabular(r">{\centering\arraybackslash}p{3cm} c"));
    assert!(
        t.contains("align: (center, center)"),
        "centering decorator must override p-column align; got:\n{t}"
    );
}

#[test]
fn raggedleft_decorator_overrides_to_right() {
    let t = typ(&wrap_tabular(r">{\raggedleft\arraybackslash}p{3cm} l"));
    assert!(
        t.contains("align: (right, left)"),
        "raggedleft decorator must map to right; got:\n{t}"
    );
}

#[test]
fn raggedright_decorator_maps_to_left() {
    let t = typ(&wrap_tabular(r">{\raggedright\arraybackslash}p{3cm} r"));
    assert!(
        t.contains("align: (left, right)"),
        "raggedright decorator must map to left; got:\n{t}"
    );
}

#[test]
fn ragged2e_raggedright_maps_to_left() {
    let t = typ(&wrap_tabular(r">{\RaggedRight\arraybackslash}p{3cm} c"));
    assert!(
        t.contains("align: (left, center)"),
        "RaggedRight decorator must map to left; got:\n{t}"
    );
}

#[test]
fn decorator_without_alignment_keeps_column_default() {
    let t = typ(&wrap_tabular(r">{\bfseries}p{3cm} c"));
    assert!(
        t.contains("align: (left, center)"),
        "non-alignment decorator must not change align; got:\n{t}"
    );
}

#[test]
fn column_count_unaffected_by_decorator() {
    // Three spec columns and a three-cell row: the decorator overrides only the
    // first column's align and must not be counted as a phantom column.
    let src = "\\documentclass{article}\\begin{document}\\begin{tabular}{>{\\centering\\arraybackslash}p{3cm} c r}\nA & B & C \\\\\n\\end{tabular}\\end{document}\n";
    let t = typ(src);
    assert!(
        t.contains("align: (center, center, right)"),
        "decorator must not add a phantom column; got:\n{t}"
    );
}
