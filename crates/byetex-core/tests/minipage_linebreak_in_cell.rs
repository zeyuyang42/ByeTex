//! Regression tests for `\\` line breaks inside a `minipage` that sits inside a
//! table cell.
//!
//! A `\\` inside a minipage is an intra-cell line break, NOT a table row
//! separator. The table emitter flattens its body and splits rows on the bare
//! `\` that `\\` emits, so a minipage-internal `\\` used to mis-split the cell
//! across rows — merging unrelated content and (downstream) leaving a `\path`
//! emitted as an escaped `\#raw(...)`. Inside a minipage, `\\` must instead
//! become a Typst `#linebreak()`. See arXiv:2605.22820's controls table.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// Two `\path` lines inside a minipage cell stay in ONE cell, joined by a
/// linebreak — not split into separate table rows, and no `\#raw` corruption.
#[test]
fn minipage_double_backslash_is_linebreak_not_row_split() {
    let src = "\\begin{tabular}{ll}\n\
        \\begin{minipage}[t]{\\linewidth}\\path|a_1|\\\\\\path|b_1|\\end{minipage} & def \\\\\n\
        \\path|c_1| & def2 \\\\\n\
        \\end{tabular}";
    let t = typst(src);

    // No escaped-hash corruption of the raw call.
    assert!(
        !t.contains("\\#raw"),
        "`\\#raw` corruption must not appear;\noutput:\n{t}"
    );
    // All three code spans survive as clean inline raw.
    for code in ["a_1", "b_1", "c_1"] {
        assert!(
            t.contains(&format!("#raw(\"{code}\")")),
            "expected clean #raw(\"{code}\");\noutput:\n{t}"
        );
    }
    // The minipage-internal `\\` becomes a Typst linebreak.
    assert!(
        t.contains("#linebreak()"),
        "minipage-internal \\\\ must become #linebreak();\noutput:\n{t}"
    );
}

/// A normal table row `\\` (outside any minipage) must STILL split rows — the
/// fix must not suppress legitimate row breaks.
#[test]
fn plain_row_break_still_splits() {
    let src = "\\begin{tabular}{ll}\na & b \\\\\nc & d \\\\\n\\end{tabular}";
    let t = typst(src);
    // The bare cells must be present and the table well-formed (2 columns).
    assert!(t.contains("columns: 2"), "2-column table;\noutput:\n{t}");
    // No spurious #linebreak() injected for ordinary rows.
    assert!(
        !t.contains("#linebreak()"),
        "ordinary row breaks must not become #linebreak();\noutput:\n{t}"
    );
}

/// A plain `\path` row FOLLOWED by a row whose first cell is a minipage: the
/// row-break `\\` between them must still split (the minipage's leading
/// whitespace-strip must not swallow it), and the minipage's first `#raw(...)`
/// must not fuse into `\#raw(...)`. This is the exact arXiv:2605.22820 pattern.
#[test]
fn row_break_before_minipage_cell_splits_cleanly() {
    let src = "\\begin{tabular}{ll}\n\
        \\path|r1c1| & first def \\\\\n\
        \\begin{minipage}[t]{\\linewidth}\\path|m_a|\\\\\\path|m_b|\\end{minipage} & second def \\\\\n\
        \\end{tabular}";
    let t = typst(src);

    assert!(
        !t.contains("\\#raw"),
        "row-break before minipage must not fuse into `\\#raw`;\noutput:\n{t}"
    );
    for code in ["r1c1", "m_a", "m_b"] {
        assert!(
            t.contains(&format!("#raw(\"{code}\")")),
            "expected clean #raw(\"{code}\");\noutput:\n{t}"
        );
    }
    // The first row's definition must not be merged into the minipage's cell:
    // `first def` and the minipage's `#raw("m_a")` must be in different cells.
    let r1c1 = t.find(r#"#raw("r1c1")"#).unwrap();
    let m_a = t.find(r#"#raw("m_a")"#).unwrap();
    assert!(
        t[r1c1..m_a].contains("first def"),
        "first row's definition must precede the minipage cell;\noutput:\n{t}"
    );
    assert!(
        t[r1c1..m_a].matches("], [").count() + t[r1c1..m_a].matches("],\n").count() >= 1,
        "a cell/row boundary must separate row 1 from the minipage cell;\noutput:\n{t}"
    );
}

/// A `tabular` NESTED inside a minipage must keep its OWN row breaks: the inner
/// table's `\\` must still split rows (not become `#linebreak()`), so no cells
/// are dropped or fused. Guards against `in_minipage` leaking into the inner
/// table.
#[test]
fn tabular_nested_in_minipage_keeps_row_breaks() {
    let t = typst(
        "\\begin{minipage}{\\linewidth}\n\
         \\begin{tabular}{ll}\na & b \\\\\nc & d \\\\\n\\end{tabular}\n\
         \\end{minipage}",
    );
    // Inner table must have all four cells, as a 2x2 table — not collapsed.
    for cell in ["[a]", "[b]", "[c]", "[d]"] {
        assert!(
            t.contains(cell),
            "inner table cell {cell} must survive (no row collapse);\noutput:\n{t}"
        );
    }
    // The inner table's row break must NOT have become a linebreak.
    assert!(
        !t.contains("#linebreak()"),
        "inner table row break must stay a row split, not #linebreak();\noutput:\n{t}"
    );
}

/// `\\` inside a minipage in normal (non-table) text also becomes a linebreak.
#[test]
fn minipage_linebreak_outside_table() {
    let t = typst(r"\begin{minipage}{\linewidth}line one\\line two\end{minipage}");
    assert!(
        t.contains("#linebreak()"),
        "minipage \\\\ should be a #linebreak();\noutput:\n{t}"
    );
}
