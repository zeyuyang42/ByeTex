//! Thread 5: a float with multiple captioned sub-blocks becomes a
//! `#subpar.grid(...)` — one inner figure per sub-block, each with its own
//! caption and label, the parent caption/label on the grid.
use byetex_core::convert;

fn typ(src: &str) -> String {
    byetex_core::convert(src, &Default::default()).typst
}

#[test]
fn subtables_with_main_caption_become_subpar_grid() {
    let t = typ(
        "\\begin{table}\n\
         \\caption{Ablations}\\label{tab:main}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{A}\\label{tab:a}\n\
         \\begin{tabular}{ll}x & y\\\\\\end{tabular}\\end{subtable}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{B}\\label{tab:b}\n\
         \\begin{tabular}{ll}p & q\\\\\\end{tabular}\\end{subtable}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{C}\\label{tab:c}\n\
         \\begin{tabular}{ll}m & n\\\\\\end{tabular}\\end{subtable}\n\
         \\end{table}\n\nSee \\ref{tab:a} and \\ref{tab:main}.",
    );
    assert!(t.contains("#subpar.grid("), "expected subpar.grid; got:\n{t}");
    assert!(t.contains("columns: (1fr, 1fr, 1fr)"), "expected 3 columns; got:\n{t}");
    assert!(t.contains("caption: [Ablations]"), "parent caption on grid; got:\n{t}");
    assert!(t.contains("label: <tab:main>"), "parent label on grid; got:\n{t}");
    assert!(t.contains("<tab:a>"), "sub-label a attached; got:\n{t}");
    assert!(t.contains("caption: [A]"), "sub-caption A present; got:\n{t}");
}
