//! Regression tests for LaTeX tabular column spec parsing.
//! `parse_column_spec` must handle `p{...}`, `m{...}`, `b{...}` and
//! other width-argument columns, not just `l`, `c`, `r`.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn p_column_counted_as_left_aligned() {
    // LaTeX: {p{0.28\textwidth} | p{0.24\textwidth} | p{0.4\textwidth}}
    // Must produce 3 columns, not 0. Paper 22724 regression.
    let src = r"\begin{tabular}{p{0.28\textwidth} | p{0.24\textwidth} | p{0.4\textwidth}}
a & b & c \\
\end{tabular}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 3"),
        "p-columns must be counted as 3 columns, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("columns: 0"),
        "0-column table must not appear, got: {}",
        out.typst
    );
}

#[test]
fn m_and_b_columns_counted() {
    // m{...} and b{...} are also paragraph-column variants
    let src = r"\begin{tabular}{m{0.5\textwidth}b{0.5\textwidth}}
a & b \\
\end{tabular}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 2"),
        "m+b columns must be counted as 2, got: {}",
        out.typst
    );
}

#[test]
fn at_separator_not_counted_as_column() {
    // @{...} is a column separator (no extra cell), not a data column
    let src = r"\begin{tabular}{l@{}r}
a & b \\
\end{tabular}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 2"),
        "@{{}} separator must not be counted as a column, got: {}",
        out.typst
    );
}

#[test]
fn array_decorator_prefix_not_counted() {
    // array-package `>{...}` (and `<{...}`) decorate the *next* column; they
    // are not data columns themselves. `>{\centering}p{5cm}` is ONE column,
    // not zero. Paper 22724 regression (bug #51).
    let src = r"\begin{tabular}{>{\centering}p{5cm}}
a \\
\end{tabular}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 1"),
        ">{{...}}p decorator must yield 1 column, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("columns: 0"),
        "0-column table must not appear, got: {}",
        out.typst
    );
}

#[test]
fn array_decorators_with_multiple_columns() {
    // Realistic tabularx row: a centered para column, a raw `>{}` decorator
    // before a plain column, and a trailing `<{}` decorator. 3 data columns.
    let src = r"\begin{tabularx}{\textwidth}{>{\centering\arraybackslash}p{3cm} >{\bfseries}l r<{\%}}
a & b & c \\
\end{tabularx}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 3"),
        "decorated 3-column tabularx must yield 3 columns, got: {}",
        out.typst
    );
}

#[test]
fn star_repeat_expands_column_count() {
    // array-package `*{N}{cols}` repeats `cols` N times. `*{2}{c}` is 2 columns.
    let src = r"\begin{tabular}{*{2}{c}}
a & b \\
\end{tabular}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 2"),
        "*{{2}}{{c}} must be 2 columns, got: {}",
        out.typst
    );
}

#[test]
fn star_repeat_mixed_with_plain_columns() {
    // `l*{3}{c}r` → 1 + 3 + 1 = 5 columns.
    let src = r"\begin{tabular}{l*{3}{c}r}
a & b & c & d & e \\
\end{tabular}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 5"),
        "l*{{3}}{{c}}r must be 5 columns, got: {}",
        out.typst
    );
}

#[test]
fn star_repeat_with_decorated_x_columns() {
    // The arXiv:2605.22724 shape: a bold `l` then five `*{2}{>{...}X}` groups
    // → 1 + 5*2 = 11 columns. Multicolumn header rows sum to 11, so an
    // undercount made Typst abort with "colspan exceeds available columns".
    let src = r"\begin{tabularx}{\linewidth}{>{\bfseries}l *{2}{>{\centering\arraybackslash}X} *{2}{>{\centering\arraybackslash}X} *{2}{>{\centering\arraybackslash}X} *{2}{>{\centering\arraybackslash}X} *{2}{>{\centering\arraybackslash}X}}
a & b & c & d & e & f & g & h & i & j & k \\
\end{tabularx}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 11"),
        "decorated *{{2}}{{X}}×5 + l must be 11 columns, got: {}",
        out.typst
    );
}

#[test]
fn plain_lcr_still_works() {
    let src = r"\begin{tabular}{|l|c|r|}
a & b & c \\
\end{tabular}";
    let out = convert(src);
    assert!(
        out.typst.contains("columns: 3"),
        "plain l|c|r must still be 3 columns, got: {}",
        out.typst
    );
}
