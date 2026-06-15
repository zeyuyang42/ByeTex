//! Thread 5: a float with multiple captioned sub-blocks becomes a
//! `#subpar.grid(...)` — one inner figure per sub-block, each with its own
//! caption and label, the parent caption/label on the grid.

fn typ(src: &str) -> String {
    byetex_core::convert(src, &Default::default()).typst
}

#[test]
fn subtables_with_main_caption_become_subpar_grid() {
    let t = typ("\\begin{table}\n\
         \\caption{Ablations}\\label{tab:main}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{A}\\label{tab:a}\n\
         \\begin{tabular}{ll}x & y\\\\\\end{tabular}\\end{subtable}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{B}\\label{tab:b}\n\
         \\begin{tabular}{ll}p & q\\\\\\end{tabular}\\end{subtable}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{C}\\label{tab:c}\n\
         \\begin{tabular}{ll}m & n\\\\\\end{tabular}\\end{subtable}\n\
         \\end{table}\n\nSee \\ref{tab:a} and \\ref{tab:main}.");
    assert!(
        t.contains("#subpar.grid("),
        "expected subpar.grid; got:\n{t}"
    );
    assert!(
        t.contains("columns: (1fr, 1fr, 1fr)"),
        "expected 3 columns; got:\n{t}"
    );
    assert!(
        t.contains("caption: [Ablations]"),
        "parent caption on grid; got:\n{t}"
    );
    assert!(
        t.contains("label: <tab:main>"),
        "parent label on grid; got:\n{t}"
    );
    assert!(t.contains("<tab:a>"), "sub-label a attached; got:\n{t}");
    assert!(
        t.contains("caption: [A]"),
        "sub-caption A present; got:\n{t}"
    );
}

#[test]
fn two_captionof_minipages_become_two_column_grid() {
    let t = typ("\\begin{figure}\n\
         \\begin{minipage}{0.41\\textwidth}\\includegraphics{a.png}\n\
         \\captionof{figure}{Left}\\label{fig:a}\\end{minipage}\\hfill\n\
         \\begin{minipage}{0.58\\textwidth}\\includegraphics{b.png}\n\
         \\captionof{figure}{Right}\\label{fig:b}\\end{minipage}\n\
         \\end{figure}\n\nSee \\ref{fig:a} and \\ref{fig:b}.");
    assert!(
        t.contains("#subpar.grid("),
        "expected subpar.grid; got:\n{t}"
    );
    assert!(
        t.contains("columns: (1fr, 1fr)"),
        "expected 2 columns; got:\n{t}"
    );
    assert!(
        t.contains("caption: [Left]") && t.contains("caption: [Right]"),
        "both captions present; got:\n{t}"
    );
    assert!(
        t.contains("<fig:a>") && t.contains("<fig:b>"),
        "both sub-labels attached; got:\n{t}"
    );
    assert!(t.contains("@preview/subpar"), "import emitted; got:\n{t}");
}

#[test]
fn stacked_table_then_figure_captionof_single_column() {
    let t = typ("\\begin{figure}\n\
         \\begin{tabular}{ll}x & y\\\\\\end{tabular}\n\
         \\captionof{table}{Tab cap}\\label{tab:s}\n\
         \\includegraphics{z.png}\n\
         \\captionof{figure}{Fig cap}\\label{fig:s}\n\
         \\end{figure}\n\nSee \\ref{tab:s} and \\ref{fig:s}.");
    assert!(
        t.contains("#subpar.grid("),
        "expected subpar.grid; got:\n{t}"
    );
    assert!(
        t.contains("columns: (1fr)"),
        "stacked → single column; got:\n{t}"
    );
    assert!(
        t.contains("caption: [Tab cap]") && t.contains("caption: [Fig cap]"),
        "both captions present; got:\n{t}"
    );
    assert!(
        t.contains("kind: table"),
        "table sub-block keeps kind: table; got:\n{t}"
    );
}

#[test]
fn linear_segmentation_does_not_leak_begin_end_markers() {
    // Regression: the float's own `\begin{figure}` / `\end{figure}` are AST
    // children of the float node. The linear-segmentation path must not push
    // them into a block's content run, or a panel body leaks a raw
    // `\begin{figure}` (invalid Typst). Corpus-class bug found verifying 22507.
    let t = typ("\\begin{figure}\n\
         \\begin{tabular}{ll}x & y\\\\\\end{tabular}\n\
         \\captionof{table}{T}\\label{t:a}\n\
         \\includegraphics{z.png}\n\
         \\captionof{figure}{F}\\label{f:b}\n\
         \\end{figure}\n");
    assert!(
        !t.contains("\\begin{figure}") && !t.contains("\\end{figure}"),
        "no raw begin/end markers may leak into the grid; got:\n{t}"
    );
}
