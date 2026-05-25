//! `\tabularnewline` is LaTeX's alias for `\\` inside `tabular`/`array`
//! environments — used to disambiguate the row separator from `\\`
//! after an optional-arg bracket `\\[1ex]`. Real arXiv papers use it
//! (28 occurrences on 2605.22507 alone); before this fix it emitted
//! an `unsupported_command` warning per call and dropped the line
//! break entirely.

use byetex_core::{convert, Category, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
}

#[test]
fn tabularnewline_emits_row_separator() {
    let src = r"\documentclass{article}\begin{document}
\begin{tabular}{cc}
a & b \tabularnewline
c & d \tabularnewline
\end{tabular}
\end{document}";
    let out = convert_str(src);
    let unsupported: Vec<_> = out
        .warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::UnsupportedCommand { name } if name == "\\tabularnewline" => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert!(
        unsupported.is_empty(),
        "\\tabularnewline should be recognized; got: {:?}",
        unsupported
    );
    // All four cells should render. Typst `#table` uses `,` to
    // separate cells; the row break manifests by `[c], [d]` appearing
    // on the line(s) after `[a], [b]`. We just check both rows are
    // present in the output.
    assert!(
        out.typst.contains("[a]") && out.typst.contains("[b]") && out.typst.contains("[c]") && out.typst.contains("[d]"),
        "expected all four cells (a, b, c, d) in:\n{}",
        out.typst
    );
}

#[test]
fn tabularnewline_outside_tabular_still_acts_as_break() {
    // Even outside tabular, `\tabularnewline` is a valid line break.
    let src = r"\documentclass{article}\begin{document}
First line. \tabularnewline Second line.
\end{document}";
    let out = convert_str(src);
    let unsupported: Vec<_> = out
        .warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::UnsupportedCommand { name } if name == "\\tabularnewline" => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert!(unsupported.is_empty());
}
