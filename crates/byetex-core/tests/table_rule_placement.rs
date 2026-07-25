//! The full-width "header rule" the emitter injects after a table's first row
//! is a heuristic — booktabs' `\midrule` sits there in most academic tables —
//! but it fired for ANY ruled table, including ones whose source declares no
//! full-width inner rule at all:
//!
//!   * a header ruled with `\cmidrule{2-3}` got a full-width line drawn through
//!     it on top of the partial one (`\cmidrule` appears in 23 of the 59 corpus
//!     papers, 224 times);
//!   * a plain `\toprule`/`\bottomrule` table got an inner rule LaTeX never
//!     draws.
//!
//! Scope note: these tests deliberately do NOT assert that rules land at their
//! true source positions (a `\midrule` after a two-row header, or between two
//! data groups, is still drawn after row 0). Placing rules faithfully requires a
//! row index shared with the emitter's own row split — see the backlog item.

use byetex_core::{convert, ConvertOptions};

fn tabular(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{booktabs}}\n\\begin{{document}}\n\\begin{{tabular}}{{lcc}}\n{body}\n\\end{{tabular}}\n\\end{{document}}\n")
}

/// Every `table.hline(...)` line in the output, in order, trimmed.
fn hlines(typst: &str) -> Vec<String> {
    typst
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("table.hline("))
        .map(|l| l.trim_end_matches(',').to_string())
        .collect()
}

#[test]
fn cmidrule_alone_does_not_add_a_full_width_rule() {
    let out = convert(
        &tabular(
            "\\toprule\nName & A & B \\\\\n\\cmidrule(lr){2-3}\nx & 1 & 2 \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let rules = hlines(&out.typst);
    assert_eq!(
        rules.len(),
        3,
        "expected top + partial + bottom, got {rules:#?}\n{}",
        out.typst
    );
    assert!(
        rules[1].contains("start: 1") && rules[1].contains("end: 3"),
        "the header rule should be the PARTIAL cmidrule only: {rules:#?}"
    );
}

#[test]
fn a_toprule_bottomrule_only_table_gets_no_inner_rule() {
    let out = convert(
        &tabular("\\toprule\nx & 1 & 2 \\\\\ny & 3 & 4 \\\\\n\\bottomrule"),
        &ConvertOptions::default(),
    );
    let rules = hlines(&out.typst);
    assert_eq!(
        rules.len(),
        2,
        "a \\toprule/\\bottomrule-only table should have exactly 2 rules: {rules:#?}\n{}",
        out.typst
    );
}

#[test]
fn a_classic_booktabs_table_is_unchanged() {
    let out = convert(
        &tabular("\\toprule\nName & A & B \\\\\n\\midrule\nx & 1 & 2 \\\\\n\\bottomrule"),
        &ConvertOptions::default(),
    );
    let rules = hlines(&out.typst);
    assert_eq!(rules.len(), 3, "expected 3 rules: {rules:#?}\n{}", out.typst);
    assert!(rules[0].contains("0.08em"), "heavy top rule: {rules:#?}");
    assert!(rules[1].contains("0.05em"), "light header rule: {rules:#?}");
    assert!(rules[2].contains("0.08em"), "heavy bottom rule: {rules:#?}");
}

#[test]
fn cmidrule_alongside_a_midrule_keeps_both() {
    let out = convert(
        &tabular(
            "\\toprule\nName & A & B \\\\\n\\midrule\n\\cmidrule(lr){2-3}\nx & 1 & 2 \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let rules = hlines(&out.typst);
    assert!(
        rules.iter().any(|r| r.contains("start: 1")),
        "partial rule lost: {rules:#?}\n{}",
        out.typst
    );
    assert!(
        rules.iter().any(|r| !r.contains("start:") && r.contains("0.05em")),
        "the \\midrule's full-width rule lost: {rules:#?}\n{}",
        out.typst
    );
}

#[test]
fn an_hline_table_keeps_its_rules() {
    let out = convert(
        &tabular("\\hline\nName & A & B \\\\\n\\hline\nx & 1 & 2 \\\\\n\\hline"),
        &ConvertOptions::default(),
    );
    assert_eq!(
        hlines(&out.typst).len(),
        3,
        "\\hline tables keep the three-rule shape:\n{}",
        out.typst
    );
}

#[test]
fn an_unruled_tabular_still_draws_nothing() {
    let out = convert(
        &tabular("x & 1 & 2 \\\\\ny & 3 & 4"),
        &ConvertOptions::default(),
    );
    assert!(
        hlines(&out.typst).is_empty(),
        "an unruled tabular must stay unruled:\n{}",
        out.typst
    );
}
