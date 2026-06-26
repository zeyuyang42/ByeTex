//! `\textbf{}`/`\emph{}` in a table cell rendered as `*…*`/`_…_` markup, which
//! the per-cell escape pass (`escape_text_cell`) then escaped to literal `\*…\*`
//! — bold headers showed visible asterisks instead of bold. Inside a table cell,
//! emit the boundary-independent function form `#strong[…]`/`#emph[…]`, which
//! survives the escape pass. Found by the visual grader on 2605.22786 (every
//! result table's bold headers leaked asterisks).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn bold_header_is_strong_not_escaped_asterisks() {
    let t = typ(r"\begin{tabular}{ll}\textbf{Name} & \textbf{Age}\\Bob & 5\end{tabular}");
    assert!(t.contains("#strong[Name]"), "bold header should be #strong[]; got:\n{t}");
    assert!(!t.contains("\\*Name"), "bold markers escaped to literal asterisks; got:\n{t}");
}

#[test]
fn emph_cell_is_emph_not_escaped_underscore() {
    let t = typ(r"\begin{tabular}{ll}\emph{lr} & b\\c & d\end{tabular}");
    assert!(t.contains("#emph[lr]"), "emph cell should be #emph[]; got:\n{t}");
    assert!(!t.contains("\\_lr"), "emph markers escaped to literal underscores; got:\n{t}");
}

#[test]
fn bold_outside_table_still_uses_shorthand() {
    // Body text keeps the compact `*…*` form (no escape pass there).
    let t = typ(r"A \textbf{bold} word.");
    assert!(t.contains("*bold*"), "body bold should stay shorthand; got:\n{t}");
}
