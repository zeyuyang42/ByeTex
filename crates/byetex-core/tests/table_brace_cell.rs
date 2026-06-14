//! Regression: a brace-wrapped table cell like `{0.131\small{\textpm 0.034}}`
//! (corpus 2605.22507) leaked literal braces and dropped its uncertainty value.
//! Two fixes combine here: font-size switches (`\small`) now render their
//! tree-sitter-absorbed `{...}` argument instead of dropping it, `\textpm` maps
//! to `±`, and a cell wholly wrapped in `{...}` has its (invisible) grouping
//! braces stripped.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn brace_wrapped_uncertainty_cell_renders() {
    let out = convert(
        "\\begin{tabular}{ll}\nMethod & {0.131\\small{\\textpm  0.034}} \\\\\n\\end{tabular}\n",
    );
    assert!(
        out.typst.contains("0.131") && out.typst.contains("0.034"),
        "both the value and uncertainty must survive, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains('±'),
        "`\\textpm` must render as ±, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("[{0.131") && !out.typst.contains("0.034}]"),
        "the cell's grouping braces must be stripped, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\small"),
        "the `\\small` directive must not leak, got: {}",
        out.typst
    );
}

#[test]
fn textpm_maps_to_plus_minus_in_text() {
    let out = convert("x \\textpm 0.5");
    assert!(out.typst.contains('±'), "expected ±, got: {}", out.typst);
}

#[test]
fn plain_cell_unchanged() {
    // Regression guard: a non-braced cell is untouched.
    let out = convert("\\begin{tabular}{ll}\na & b \\\\\n\\end{tabular}\n");
    assert!(
        out.typst.contains("[a], [b]"),
        "plain cells must be unchanged, got: {}",
        out.typst
    );
}

#[test]
fn two_groups_in_cell_concatenate() {
    // `{a}{b}` is two bare grouping (scoping) groups — each emits its inner
    // content WITHOUT braces, so the cell renders `ab` (the correct LaTeX
    // concatenation). The old behavior preserved literal `{a}{b}`, which is a
    // Typst code block that does not compile.
    let out = convert("\\begin{tabular}{l}\n{a}{b} \\\\\n\\end{tabular}\n");
    assert!(
        out.typst.contains("[ab]"),
        "two-group cell must concatenate to `ab`, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("{a}") && !out.typst.contains("{b}"),
        "no literal grouping braces may reach Typst, got: {}",
        out.typst
    );
}
