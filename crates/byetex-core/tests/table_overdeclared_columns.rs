//! Regression (corpus 2605.31561): a tabular whose column spec declares MORE
//! columns than any row actually uses (`{llrrrrrrrrrrrrrr}` = 16 but the rows,
//! with `\multicolumn{3}` groups, only occupy 11) made byetex emit
//! `#table(columns: 16, ...)`. Typst then rejected the colspan layout with
//! "cell's colspan would cause it to exceed the available column(s)". The
//! column count must be clamped to the actual max row occupancy.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn overdeclared_spec_clamps_to_actual_row_width() {
    // spec declares 5 columns; every row uses 2 → columns: 2.
    let out = convert("\\begin{tabular}{lllll}\na & b \\\\\nc & d \\\\\n\\end{tabular}\n");
    assert!(out.typst.contains("columns: 2"), "expected columns: 2, got:\n{}", out.typst);
    assert!(!out.typst.contains("columns: 5"), "spec count 5 must be clamped, got:\n{}", out.typst);
}

#[test]
fn multicolumn_over_overdeclared_spec_clamps() {
    // spec 6; the only row is `\multicolumn{2}{c}{X} & b & c` → occupies 4
    // columns (colspan 2 + 1 + 1). columns must be 4, not 6.
    let out = convert(
        "\\begin{tabular}{cccccc}\n\\multicolumn{2}{c}{X} & b & c \\\\\n\\end{tabular}\n",
    );
    assert!(out.typst.contains("columns: 4"), "expected columns: 4, got:\n{}", out.typst);
}

#[test]
fn well_formed_table_column_count_unchanged() {
    // Regression guard: a spec matching its content is untouched.
    let out = convert("\\begin{tabular}{lc}\na & b \\\\\nc & d \\\\\n\\end{tabular}\n");
    assert!(out.typst.contains("columns: 2"), "well-formed columns: 2, got:\n{}", out.typst);
}
