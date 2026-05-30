//! Regression tests for the `path` package's verb-like `\path|...|` command.
//!
//! `\path` (path.sty) typesets its delimited argument verbatim, allowing line
//! breaks — analogous to `\verb`. tree-sitter parses `\path` as a generic
//! command followed by the `|...|`-delimited text, so without explicit handling
//! it emitted an `unsupported_command` warning and dropped the content. This is
//! common inside tables (e.g. feature/column names). See arXiv:2605.22820.

use byetex_core::{convert, ConvertOptions};

fn convert_src(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

/// `\path|name|` must emit the delimited content as `#raw(...)`, not drop it.
#[test]
fn path_pipe_delimited_becomes_raw() {
    let out = convert_src(r"\path|on_promo|");
    assert!(
        out.typst.contains("#raw(\"on_promo\")"),
        "\\path|...| must emit #raw(\"on_promo\");\noutput:\n{}",
        out.typst
    );
}

/// `\path|...|` must not produce an `unsupported_command` warning for `\path`.
#[test]
fn path_emits_no_unsupported_warning() {
    let out = convert_src(r"\path|week_rank|");
    let has_path_warning = out
        .warnings
        .iter()
        .any(|w| format!("{:?}", w.category).contains("\\path"));
    assert!(
        !has_path_warning,
        "\\path must not warn as unsupported;\nwarnings:\n{:#?}",
        out.warnings
    );
}

/// A non-pipe delimiter (`\path!...!`) must work too — path.sty allows any
/// non-letter delimiter, same as `\verb`.
#[test]
fn path_alternate_delimiter() {
    let out = convert_src(r"\path!sin_52!");
    assert!(
        out.typst.contains("#raw(\"sin_52\")"),
        "\\path with `!` delimiter must emit #raw(\"sin_52\");\noutput:\n{}",
        out.typst
    );
}

/// The tikz `\path (a) -- (b);` form must NOT be treated as verbatim: when the
/// byte after the command is whitespace / `(` / `{` / `[`, `\path` falls through
/// to the unsupported-command warning rather than swallowing source as a
/// `#raw(...)` delimiter run. (tikzpicture bodies are dropped elsewhere, but the
/// guard must hold even for a bare `\path` outside a picture.)
#[test]
fn path_tikz_form_is_not_verbatim() {
    for src in [
        r"\path (0,0) -- (1,1);",
        r"\path[draw] (0,0) circle (1);",
        r"\path{foo}",
    ] {
        let out = convert_src(src);
        assert!(
            !out.typst.contains("#raw("),
            "tikz-style `{src}` must not be emitted as #raw(...);\noutput:\n{}",
            out.typst
        );
    }
}

/// `\path` inside a table cell is recovered as `#raw(...)` rather than dropped.
/// (Underscores inside `#raw("...")` in a table cell are escaped to `\_` by a
/// pre-existing table-escaping pass that also affects `\verb`/`\texttt`; that
/// is tracked separately, so this test asserts only what `\path` support
/// guarantees: the cells are recovered as raw and no `\path` warning fires.)
#[test]
fn path_inside_tabular_cell() {
    let src = r"\begin{tabular}{ll}
\path|feature_a| & 0.5 \\
\path|feature_b| & 0.7 \\
\end{tabular}";
    let out = convert_src(src);
    let raw_cells = out.typst.matches("#raw(").count();
    assert!(
        raw_cells >= 2,
        "both \\path cells must be recovered as #raw(...), found {raw_cells};\noutput:\n{}",
        out.typst
    );
    let has_path_warning = out
        .warnings
        .iter()
        .any(|w| format!("{:?}", w.category).contains("\\path"));
    assert!(
        !has_path_warning,
        "\\path in a table must not warn as unsupported;\nwarnings:\n{:#?}",
        out.warnings
    );
}
