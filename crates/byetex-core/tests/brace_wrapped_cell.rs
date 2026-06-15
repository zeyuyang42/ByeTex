//! Bug (corpus 2605.31203 siunitx table; also the 22507 `\small`/`\textpm`
//! leak): a table cell wrapped in braces — `{$\Braket{...}$}`, `{\small ...}` —
//! leaked as RAW source. emit_tabular collected the body with a filter that
//! dropped EVERY `curly_group` (intending only the column-spec group), so a
//! brace-wrapped cell was never emitted through the converter and the parent
//! gap-copy spilled its raw LaTeX. Only the leading spec/width group(s) must be
//! skipped; cell-level `{...}` groups must convert normally.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn brace_wrapped_math_cell_is_converted() {
    let t = typ("\\begin{tabular}{cc}\nA & {$\\alpha$} \\\\\n\\end{tabular}\n");
    // `\alpha` must convert to `alpha`, not leak as raw `\alpha`.
    assert!(
        t.contains("alpha") && !t.contains("\\alpha"),
        "brace-wrapped math cell must be converted; got:\n{t}"
    );
}

#[test]
fn brace_wrapped_command_cell_is_converted() {
    // The 31203 shape: `{$\Braket{...}$}` inside `\usepackage{braket}`.
    let t = typ(
        "\\documentclass{article}\\usepackage{braket}\\begin{document}\n\
         \\begin{tabular}{cc}\nA & {$\\Braket{x}$} \\\\\n\\end{tabular}\n\\end{document}",
    );
    assert!(
        !t.contains("\\Braket"),
        "brace-wrapped \\Braket cell must convert, not leak; got:\n{t}"
    );
}

#[test]
fn column_spec_still_skipped_and_count_right() {
    // Regression guard: the column-spec group is still dropped (not treated as a
    // cell), so the column count and a normal table are unaffected.
    let t = typ("\\begin{tabular}{ccc}\nA & B & C \\\\\n\\end{tabular}\n");
    assert!(
        t.contains("columns: 3"),
        "column count must stay 3; got:\n{t}"
    );
    assert!(
        t.contains("[A], [B], [C]"),
        "plain cells unaffected; got:\n{t}"
    );
    assert!(
        !t.contains("[ccc]") && !t.contains("[{ccc}]"),
        "spec must not become a cell; got:\n{t}"
    );
}
