//! Horizontal rules are drawn where the LaTeX source declares them.
//!
//! Backlog O1. The emitter used to draw the canonical booktabs shape (top / one
//! rule after the first row / bottom) regardless of the source, so every
//! mid-table `\midrule` group separator collapsed into one — corpus 2605.22821
//! declares 91 full-width rules and got 45.
//!
//! The obvious fix — a second scan of the raw source counting `\\` — is
//! unsound: it misses `\tabularnewline` (a full `\\` synonym), counts `\\`
//! inside `\makecell{a \\ b}` (a line break within ONE cell), and cannot see
//! `\newline` in a cell making emitted rows outnumber source breaks. Instead
//! the rule commands now emit sentinels into the rendered body, so the row
//! split that positions them IS the emitter's own row split.

use byetex_core::{convert, ConvertOptions};

fn tabular(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{booktabs}}\n\\usepackage{{makecell}}\n\\begin{{document}}\n\\begin{{tabular}}{{lcc}}\n{body}\n\\end{{tabular}}\n\\end{{document}}\n")
}

/// `(rows emitted before it, the rule line)` for every rule, in order.
fn rule_positions(typst: &str) -> Vec<(usize, String)> {
    let mut rows = 0usize;
    let mut out = Vec::new();
    for line in typst.lines().map(str::trim) {
        if line.starts_with("table.hline(") {
            out.push((rows, line.trim_end_matches(',').to_string()));
        } else if line.starts_with('[') || line.starts_with("table.cell(") {
            rows += 1;
        }
    }
    out
}

fn full_rules(typst: &str) -> Vec<(usize, String)> {
    rule_positions(typst)
        .into_iter()
        .filter(|(_, l)| !l.contains("start:"))
        .collect()
}

/// No sentinel may survive into the emitted document.
fn assert_no_sentinels(typst: &str) {
    for c in ['\u{11}', '\u{12}', '\u{13}', '\u{14}'] {
        assert!(
            !typst.contains(c),
            "rule sentinel {:?} leaked into the output:\n{}",
            c,
            typst
        );
    }
}

#[test]
fn a_midrule_between_data_groups_lands_between_them() {
    let out = convert(
        &tabular(
            "\\toprule\nName & A & B \\\\\n\\midrule\nx & 1 & 2 \\\\\n\\midrule\ny & 3 & 4 \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let rules = full_rules(&out.typst);
    let at: Vec<usize> = rules.iter().map(|(r, _)| *r).collect();
    assert_eq!(
        at,
        vec![0, 1, 2, 3],
        "expected top / after-header / group-separator / bottom, got {rules:#?}\n{}",
        out.typst
    );
    assert_no_sentinels(&out.typst);
}

#[test]
fn a_midrule_after_a_two_row_header_follows_the_second_row() {
    let out = convert(
        &tabular(
            "\\toprule\nGroup & A & B \\\\\nUnit & m & s \\\\\n\\midrule\nx & 1 & 2 \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(
        at,
        vec![0, 2, 3],
        "rule should follow the 2-row header, not the 1st row:\n{}",
        out.typst
    );
}

/// `\\` inside a braced group is a line break within ONE cell, not a row
/// separator. Corpus 2605.22821: 14 of its 15 tables use two-line `\makecell`
/// headers, and every rule after such a header landed a row late.
#[test]
fn a_linebreak_inside_makecell_does_not_shift_the_rules() {
    let out = convert(
        &tabular(
            "\\toprule\n\\makecell{Vocab \\\\ size} & A & B \\\\\n\\midrule\nx & 1 & 2 \\\\\n\\midrule\ny & 3 & 4 \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(
        at,
        vec![0, 1, 2, 3],
        "the \\makecell line break was counted as a row break:\n{}",
        out.typst
    );
}

/// `\tabularnewline` is a full `\\` synonym — 28 occurrences on corpus
/// 2605.22507 alone. A raw-byte `\\` scan misses it entirely.
#[test]
fn tabularnewline_rows_position_rules_correctly() {
    let out = convert(
        &tabular(
            "\\toprule\nName & A & B \\tabularnewline\n\\midrule\nx & 1 & 2 \\tabularnewline\ny & 3 & 4 \\tabularnewline\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(
        at,
        vec![0, 1, 3],
        "\\tabularnewline rows mis-positioned the rules:\n{}",
        out.typst
    );
}

#[test]
fn a_cmidrule_lands_on_the_row_it_follows() {
    let out = convert(
        &tabular(
            "\\toprule\n\\makecell{Vocab \\\\ size} & A & B \\\\\n\\cmidrule(lr){2-3}\nx & 1 & 2 \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let partial: Vec<(usize, String)> = rule_positions(&out.typst)
        .into_iter()
        .filter(|(_, l)| l.contains("start:"))
        .collect();
    assert_eq!(partial.len(), 1, "expected one partial rule: {partial:#?}");
    assert_eq!(partial[0].0, 1, "cmidrule follows the header row: {partial:#?}\n{}", out.typst);
    assert!(
        partial[0].1.contains("start: 1") && partial[0].1.contains("end: 3"),
        "cmidrule span wrong: {partial:#?}"
    );
}

#[test]
fn a_last_row_without_a_trailing_break_still_gets_its_bottom_rule() {
    let out = convert(
        &tabular("\\toprule\nName & A & B \\\\\n\\midrule\nx & 1 & 2\n\\\\\n\\bottomrule"),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(
        *at.last().unwrap(),
        2,
        "the bottom rule must sit after the LAST row:\n{}",
        out.typst
    );
}

#[test]
fn a_cmidrule_only_header_still_gets_no_full_width_rule() {
    // The 0.7.1 fix must survive the redesign.
    let out = convert(
        &tabular(
            "\\toprule\nName & A & B \\\\\n\\cmidrule(lr){2-3}\nx & 1 & 2 \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(at, vec![0, 2], "no full-width header rule belongs here:\n{}", out.typst);
}

#[test]
fn a_classic_booktabs_table_is_unchanged() {
    let out = convert(
        &tabular("\\toprule\nName & A & B \\\\\n\\midrule\nx & 1 & 2 \\\\\n\\bottomrule"),
        &ConvertOptions::default(),
    );
    let rules = full_rules(&out.typst);
    let at: Vec<usize> = rules.iter().map(|(r, _)| *r).collect();
    assert_eq!(at, vec![0, 1, 2], "{rules:#?}\n{}", out.typst);
    assert!(rules[0].1.contains("0.08em"), "heavy top: {rules:#?}");
    assert!(rules[1].1.contains("0.05em"), "light mid: {rules:#?}");
    assert!(rules[2].1.contains("0.08em"), "heavy bottom: {rules:#?}");
}

#[test]
fn an_hline_table_rules_every_declared_position() {
    let out = convert(
        &tabular("\\hline\nName & A & B \\\\\n\\hline\nx & 1 & 2 \\\\\n\\hline"),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(at, vec![0, 1, 2], "one rule per \\hline:\n{}", out.typst);
    assert_no_sentinels(&out.typst);
}

#[test]
fn a_doubled_hline_at_the_top_collapses_to_one_top_rule() {
    let out = convert(
        &tabular("\\hline\\hline\nName & A & B \\\\\nx & 1 & 2 \\\\\n\\hline"),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(at, vec![0, 2], "double top rule mis-filed:\n{}", out.typst);
}

/// The row-break `\` the emitter writes is only a break when followed by
/// WHITESPACE. A sentinel glued straight onto it swallowed the break and merged
/// two rows into one (corpus 2605.31063, caught by the acceptance gate).
#[test]
fn a_rule_immediately_after_a_row_break_does_not_swallow_it() {
    let out = convert(
        // No space between the `\\` and the `\midrule`.
        &tabular("\\toprule\nA & B & C \\\\\\midrule\nx & 1 & 2 \\\\\n\\bottomrule"),
        &ConvertOptions::default(),
    );
    let rows = out
        .typst
        .lines()
        .filter(|l| l.trim_start().starts_with('['))
        .count();
    assert_eq!(rows, 2, "the row break was swallowed:\n{}", out.typst);
    assert_no_sentinels(&out.typst);
}

/// A row where every column is covered by an active rowspan collapses to no
/// cells — the standard `\multirow` placeholder row. It still OWNS the boundary
/// after it, so skipping the whole iteration dropped the `\bottomrule` of any
/// table whose last row is a placeholder.
#[test]
fn a_collapsed_multirow_placeholder_row_still_emits_its_rules() {
    let out = convert(
        &format!(
            "\\documentclass{{article}}\n\\usepackage{{booktabs}}\n\\usepackage{{multirow}}\n\\begin{{document}}\n\
             \\begin{{tabular}}{{cc}}\n\\toprule\na & b \\\\\n\\midrule\n\
             \\multirow{{2}}{{*}}{{X}} & \\multirow{{2}}{{*}}{{Y}} \\\\\n & \\\\\n\\bottomrule\n\
             \\end{{tabular}}\n\\end{{document}}\n"
        ),
        &ConvertOptions::default(),
    );
    let rules = full_rules(&out.typst);
    assert_eq!(
        rules.len(),
        3,
        "expected top / mid / bottom, got {rules:#?}\n{}",
        out.typst
    );
    assert!(
        rules.last().unwrap().1.contains("0.08em"),
        "the bottom rule was dropped by the collapsed placeholder row: {rules:#?}\n{}",
        out.typst
    );
}

/// A cell rendering a literal `$` (from `\verb`/`\texttt{\$…}`, which becomes
/// `#raw("$PATH")`) is not a cut math span, and must not suppress a `\midrule`
/// the source really declared.
#[test]
fn a_literal_dollar_in_a_cell_does_not_suppress_a_declared_rule() {
    let out = convert(
        &tabular(
            "\\toprule\nVar & Value \\\\\n\\texttt{\\$PATH} & /usr/bin \\\\\n\\midrule\na & b \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(
        at,
        vec![0, 2, 3],
        "the \\midrule was suppressed by a literal `$`:\n{}",
        out.typst
    );
}

/// `\hline` as verbatim CELL CONTENT (`\verb|\hline|`) is text, not a rule
/// declaration — synthesizing a rule from it draws a line LaTeX never does.
#[test]
fn a_verbatim_hline_in_a_cell_draws_no_rule() {
    let out = convert(
        &tabular(
            "\\toprule\nA & B \\\\\n\\midrule\n1 & 2 \\\\\n3 & \\verb|\\hline| \\\\\n4 & 5 \\\\\n\\bottomrule",
        ),
        &ConvertOptions::default(),
    );
    let at: Vec<usize> = full_rules(&out.typst).iter().map(|(r, _)| *r).collect();
    assert_eq!(
        at,
        vec![0, 1, 4],
        "a verbatim \\hline synthesized a spurious rule:\n{}",
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
        full_rules(&out.typst).is_empty(),
        "an unruled tabular must stay unruled:\n{}",
        out.typst
    );
}
