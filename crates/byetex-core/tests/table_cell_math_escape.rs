//! Bug: `escape_text_cell` tracks `in_math` for `#` but NOT for the
//! `_`/`*`/`@`/`<`/backtick markup escapes, so a subscript inside an inline
//! `$...$` math cell — `A & $y_w$` — was rewritten to `$y\_w$`. In Typst math
//! `\_` is a literal underscore, not a subscript, so the cell renders wrong
//! (and, combined with other math, can break compilation). Math content in a
//! cell is already converted Typst math and must pass through verbatim.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn subscript_in_table_cell_math_is_not_escaped() {
    let t = typ("\\begin{tabular}{cc}\nA & $y_w$ \\\\\n\\end{tabular}\n");
    // The subscript must survive as `y_w`, not be escaped to `y\_w`.
    assert!(
        t.contains("$y_w$"),
        "subscript in a math cell must stay `y_w`; got:\n{t}"
    );
    assert!(
        !t.contains("y\\_w"),
        "math-mode `_` must NOT be escaped inside a cell; got:\n{t}"
    );
}

#[test]
fn text_mode_underscore_in_cell_still_escaped() {
    // Regression guard: a literal `_` OUTSIDE math (plain text cell) must
    // still be escaped, or Typst markup treats it as emphasis.
    let t = typ("\\begin{tabular}{cc}\nA & file_name \\\\\n\\end{tabular}\n");
    assert!(
        t.contains("file\\_name"),
        "text-mode `_` must still be escaped; got:\n{t}"
    );
}

#[test]
fn superscript_star_in_cell_math_is_not_escaped() {
    // `*` is valid in Typst math (e.g. a superscript star `h^*`); it must not
    // be escaped to `\*` inside a cell's math.
    let t = typ("\\begin{tabular}{cc}\nA & $h^*$ \\\\\n\\end{tabular}\n");
    assert!(
        t.contains("$h^*$"),
        "math-mode `*` must stay literal inside a cell; got:\n{t}"
    );
}
