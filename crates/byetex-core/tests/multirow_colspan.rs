//! Regression tests for `\multirow` rowspan placeholder suppression and
//! `\multicolumn` / `\multirow` combined colspan/rowspan tracking.
//!
//! LaTeX `\multirow{N}{*}{X}` spans N rows. In Typst, `table.cell(rowspan: N)`
//! automatically occupies its position in the subsequent N-1 rows.  ByeTex
//! was previously emitting an extra empty `[]` placeholder cell for the
//! covered rows, which shifted subsequent cells and caused a
//! "cell's colspan would exceed available columns" error.

use byetex_core::{convert, ConvertOptions};

/// Helper: run convert with defaults and return the Typst string.
fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// A `\multirow{2}{*}{A}` table at column 1 must NOT emit an empty `[]`
/// placeholder for the covered second row.  The second row should have
/// exactly 2 cells: `[D]` and `[E]`.
#[test]
fn multirow_no_placeholder_cell_in_covered_row() {
    let src = r"
\begin{document}
\begin{table}
\begin{tabular}{lcc}
\multirow{2}{*}{A} & B & C \\
                   & D & E \\
\end{tabular}
\end{table}
\end{document}
";
    let t = typst(src);

    // Row 1: A (rowspan:2), B, C — 3 cells
    assert!(
        t.contains("table.cell(rowspan: 2)[A]"),
        "rowspan cell missing;\noutput:\n{t}"
    );
    // Row 2: the placeholder for col 1 must be absent; only D and E
    assert!(
        !t.contains("[D], [E]")
            || !t.contains("[], [D]"),
        "empty placeholder [] still present before [D];\noutput:\n{t}"
    );
    // Specifically: the second row must NOT start with `[]`
    // The pattern "table.cell(rowspan: 2)[A], [B], [C],\n  [], [D]"
    // indicates the bug is present.
    assert!(
        !t.contains("[], [D]"),
        "rowspan placeholder not suppressed — got [], [D] in second row;\noutput:\n{t}"
    );
}

/// A table that combines `\multirow` with `\multicolumn` after: ByeTex
/// must not emit placeholder cells that shift the `colspan` cell off-column,
/// causing a Typst "colspan exceeds available columns" error.
///
/// This is the root cause seen in arXiv:2605.22584.
#[test]
fn multirow_then_multicolumn_no_colspan_overflow() {
    let src = r"
\begin{document}
\begin{table}
\begin{tabular}{lcc}
Header1 & Header2 & Header3 \\
\multirow{2}{*}{X} & 1 & 2 \\
                   & 3 & 4 \\
\multicolumn{3}{l}{Footer} \\
\end{tabular}
\end{table}
\end{document}
";
    let t = typst(src);

    // Footer row must be a colspan: 3 cell at column 1 (no preceding []).
    assert!(
        t.contains("table.cell(colspan: 3)[Footer]"),
        "colspan footer cell missing;\noutput:\n{t}"
    );
    // No empty placeholder should precede the colspan cell.
    assert!(
        !t.contains("[], table.cell(colspan: 3)"),
        "empty placeholder before colspan cell — would overflow;\noutput:\n{t}"
    );
    // Second row of multirow should not have the empty placeholder cell.
    assert!(
        !t.contains("[], [3]"),
        "rowspan placeholder not suppressed in second row;\noutput:\n{t}"
    );
}

/// `\multirow` at a non-first column: only that column's placeholder is
/// suppressed; other columns in the covered rows are emitted normally.
#[test]
fn multirow_at_middle_column_suppresses_correct_placeholder() {
    let src = r"
\begin{document}
\begin{table}
\begin{tabular}{ccc}
A & \multirow{2}{*}{B} & C \\
D &                    & F \\
\end{tabular}
\end{table}
\end{document}
";
    let t = typst(src);

    // Row 1: A, B (rowspan:2), C
    assert!(
        t.contains("table.cell(rowspan: 2)[B]"),
        "mid-column rowspan cell missing;\noutput:\n{t}"
    );
    // Row 2: D and F — the empty placeholder for col 2 must be absent.
    assert!(
        !t.contains("[], [F]"),
        "placeholder for middle column not suppressed;\noutput:\n{t}"
    );
    // D must appear in the second row output.
    assert!(
        t.contains("[D]"),
        "[D] missing from second row;\noutput:\n{t}"
    );
    assert!(
        t.contains("[F]"),
        "[F] missing from second row;\noutput:\n{t}"
    );
}
