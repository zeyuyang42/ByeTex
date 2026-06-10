//! Regression (corpus 2605.31203): a tabular row with FEWER cells than the
//! column count must be padded to the full width. LaTeX pads short rows to the
//! column count implicitly; Typst's `table()` auto-placement flows cells
//! continuously, so an un-padded short row shifts every following cell left.
//! Eventually a later `\multicolumn{N}` colspan lands where fewer than `N`
//! columns remain → Typst aborts with "cell's colspan would cause it to exceed
//! the available column(s)". Padding short rows with empty cells keeps every
//! logical row aligned to the grid.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn short_row_is_padded_to_full_width() {
    // 3-column table; the middle row has a single cell. It must be padded to
    // three column slots so the following `\multicolumn{3}` starts a fresh row.
    let out = convert(
        "\\begin{tabular}{lll}\na & b & c \\\\\nshort \\\\\n\\multicolumn{3}{c}{wide} \\\\\n\\end{tabular}\n",
    );
    assert!(
        out.typst.contains("[short], [], []"),
        "short row must be padded to 3 columns, got:\n{}",
        out.typst
    );
}

#[test]
fn short_row_before_multicolumn_does_not_overflow() {
    // The realistic 2605.31203 shape: a lone-cell label row, then data rows,
    // then a `\multicolumn{3}` spanning row. After padding, the colspan-3 cell
    // must begin its own row (preceded by a row terminator), never chaining
    // onto a partially-filled row.
    let out = convert(
        "\\begin{tabular}{lllll}\nlabel \\\\\nHF & 1 & 2 & 3 & 4 \\\\\n\\multicolumn{3}{c}{group} \\\\\n\\end{tabular}\n",
    );
    // The lone label cell is padded out to five columns.
    assert!(
        out.typst.contains("[label], [], [], [], []"),
        "lone label row must be padded to 5 columns, got:\n{}",
        out.typst
    );
}

#[test]
fn well_formed_table_gains_no_padding() {
    // Regression guard: a table whose every row already fills the columns must
    // not gain any spurious empty cells.
    let out = convert("\\begin{tabular}{lc}\na & b \\\\\nc & d \\\\\n\\end{tabular}\n");
    assert!(
        !out.typst.contains("[]"),
        "well-formed table must not gain empty padding cells, got:\n{}",
        out.typst
    );
}
