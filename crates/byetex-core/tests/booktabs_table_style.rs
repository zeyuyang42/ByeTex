//! Design fidelity: byetex emitted `#table(columns, align)` with no stroke, so
//! Typst drew its DEFAULT full grid (a line around every cell) — but academic
//! papers (42/56 of the corpus) use booktabs: three horizontal rules (top /
//! after-header / bottom) and NO vertical lines. And a LaTeX tabular with no
//! rule commands draws no lines at all. So:
//!   * `stroke: none` always (kills the spurious grid; matches a rule-less
//!     tabular, which has no lines);
//!   * booktabs `table.hline()` rules only when the source used rule commands.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn booktabs_table_gets_stroke_none_and_three_rules() {
    let src = "\\begin{tabular}{cc}\n\
        \\toprule\nName & Value \\\\\n\\midrule\nA & 1 \\\\\nB & 2 \\\\\n\\bottomrule\n\
        \\end{tabular}\n";
    let t = typ(src);
    assert!(t.contains("stroke: none"), "table must disable the default grid; got:\n{t}");
    let rules = t.matches("table.hline(").count();
    assert_eq!(rules, 3, "booktabs table needs 3 rules (top/header/bottom); got {rules}:\n{t}");
}

#[test]
fn ruleless_tabular_has_no_lines() {
    // A tabular with no rule commands draws no lines in LaTeX — and must not
    // get Typst's grid NOR spurious booktabs rules.
    let src = "\\begin{tabular}{cc}\nA & 1 \\\\\nB & 2 \\\\\n\\end{tabular}\n";
    let t = typ(src);
    assert!(t.contains("stroke: none"), "rule-less table must have no grid; got:\n{t}");
    assert!(
        !t.contains("table.hline("),
        "rule-less table must get no horizontal rules; got:\n{t}"
    );
}

#[test]
fn hline_table_is_treated_as_ruled() {
    // Old-style `\hline` tables count as ruled (get the booktabs frame).
    let src = "\\begin{tabular}{cc}\n\\hline\nName & Value \\\\\n\\hline\nA & 1 \\\\\n\\hline\n\\end{tabular}\n";
    let t = typ(src);
    assert!(t.contains("stroke: none"), "got:\n{t}");
    assert!(t.contains("table.hline("), "an \\hline table must get rules; got:\n{t}");
}
