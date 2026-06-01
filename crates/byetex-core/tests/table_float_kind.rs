//! Phase 2c / defect D1: a `\begin{table}` float whose body is a `tabular`
//! must be emitted as `#figure(table(...), kind: table, ...)` so Typst captions
//! it "Table N" (not "Figure N"); and the tabular must still be found when it
//! arrives via `\input{...}` from a separate file (the common pattern that was
//! dropping the whole table with a "figure has no tabular body" warning).
//!
//! Root cause (corpus 2605.22776 dropped 7/8 tables, 2605.22817 7/9): the float
//! emitter set `kind: table` only for `\captionof{table}` (never for a `table`
//! env with a real `\caption`), and scanned only AST children for the tabular,
//! so an `\input`-ed tabular was invisible.

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, Category, ConvertOptions};
use tempfile::TempDir;

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn table_env_with_inline_tabular_and_caption_sets_kind_table() {
    // `\begin{table}` + inline tabular + real `\caption` — the common case.
    let t = typ(
        "\\begin{table}\n\\centering\n\\begin{tabular}{cc}a & b\\\\c & d\\end{tabular}\n\\caption{My table}\\label{tab:x}\n\\end{table}",
    );
    assert!(
        t.contains("kind: table"),
        "a table env wrapping a tabular must set kind: table so refs read 'Table N'; got:\n{t}"
    );
    assert!(
        t.contains("caption: [My table]"),
        "caption must be preserved; got:\n{t}"
    );
    assert!(
        t.contains("<tab:x>"),
        "label must attach; got:\n{t}"
    );
    // The tabular body must actually be present (not the placeholder rect).
    assert!(
        t.contains("table("),
        "the tabular must be rendered as a table() call; got:\n{t}"
    );
    assert!(
        !t.contains("(figure)") && !t.contains("needs manual review"),
        "must not fall back to the placeholder rect; got:\n{t}"
    );
}

#[test]
fn figure_env_with_graphics_does_not_set_kind_table() {
    // Regression guard: a real figure (image body) must NOT become kind: table.
    let t = typ(
        "\\begin{figure}\n\\includegraphics{x.png}\n\\caption{Pic}\\label{fig:p}\n\\end{figure}",
    );
    assert!(
        !t.contains("kind: table"),
        "an image figure must not be tagged kind: table; got:\n{t}"
    );
    assert!(t.contains("caption: [Pic]"), "caption preserved; got:\n{t}");
}

#[test]
fn input_ed_tabular_inside_table_float_is_not_dropped() {
    // The 22776 pattern: `\begin{table}{ \input{results} }\caption{}\end{table}`
    // where the tabular lives in a separate file.
    let tmp = TempDir::new().expect("tempdir");
    fs::write(
        tmp.path().join("results.tex"),
        "\\begin{tabular}{lc}\nName & Score\\\\\nAlpha & 1\\\\\nBeta & 2\\\\\n\\end{tabular}\n",
    )
    .unwrap();
    let main = "\\begin{table}\n\\centering\n\\input{results}\n\\caption{Results table}\\label{tab:res}\n\\end{table}\n";
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(tmp.path().to_path_buf()),
        },
    );
    let t = &out.typst;
    // The table must be rendered, not dropped to a placeholder.
    assert!(
        t.contains("table(") && t.contains("[Name]") && t.contains("[Score]"),
        "the \\input-ed tabular must be rendered inside the float; got:\n{t}"
    );
    assert!(
        t.contains("kind: table"),
        "an \\input-ed tabular float must also set kind: table; got:\n{t}"
    );
    assert!(
        t.contains("caption: [Results table]") && t.contains("<tab:res>"),
        "caption + label must survive; got:\n{t}"
    );
    // The "no tabular body" warning must NOT fire for this float.
    let dropped = out.warnings.iter().any(|w| matches!(
        &w.category,
        Category::NeedsManualReview { reason } if reason.contains("no \\includegraphics or tabular body")
    ));
    assert!(!dropped, "table must not be dropped as 'no tabular body'; warnings: {:?}",
        out.warnings.iter().map(|w| &w.category).collect::<Vec<_>>());
}

#[test]
fn rowbreak_with_length_arg_does_not_break_cells() {
    // `\\[0.85em]` is a row break with vertical-space arg. The `[0.85em]` is
    // NOT cell content; if it leaks into a cell as `[\\[0.85em\]]` the escaped
    // brackets break the Typst content block ("unclosed delimiter"). Surfaced
    // by corpus 2605.22800 once D1 stopped dropping the table. The bracketed
    // length must be stripped, not emitted as a leading cell.
    let t = typ(
        "\\begin{tabular}{cc}\na & b\\\\[0.85em]\nc & d\\\\\n\\end{tabular}",
    );
    assert!(
        !t.contains("[\\\\[0.85em") && !t.contains("[0.85em]"),
        "the \\\\[len] spacing arg must not appear as a cell; got:\n{t}"
    );
    // Both real rows' cells must survive.
    for cell in ["[a]", "[b]", "[c]", "[d]"] {
        assert!(t.contains(cell), "cell {cell} must be present; got:\n{t}");
    }
}
