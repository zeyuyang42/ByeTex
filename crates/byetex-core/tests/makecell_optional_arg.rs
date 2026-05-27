//! Regression tests for `\makecell[opts]{content}` inside tabular cells.
//! When tree-sitter-latex parses `\makecell[l]{...}` with an optional arg,
//! it places the `{content}` as an AST sibling rather than a child of the
//! generic_command node. The emit layer must find and render it via source-
//! byte peek to preserve content and process inline math.

use byetex_core::{convert, ConvertOptions};

fn empty_opts() -> ConvertOptions {
    ConvertOptions {
        source_name: None,
        base_dir: None,
    }
}

fn convert_src(src: &str) -> String {
    convert(src, &empty_opts()).typst
}

#[test]
fn makecell_content_preserved_in_table() {
    // `\makecell[l]{content}` — the content must appear in the table cell.
    let src = r"\documentclass{article}
\usepackage{makecell}
\begin{document}
\begin{tabular}{cc}
  A & \makecell[l]{Hello World}
\end{tabular}
\end{document}";
    let out = convert_src(src);
    assert!(
        out.contains("Hello World"),
        "makecell content must appear in table; got:\n{}",
        out
    );
}

#[test]
fn makecell_math_converted_inside_optional_arg() {
    // 2605.22579 regression: `\makecell[l]{Agreement $\downarrow$}` —
    // the `\downarrow` inside \makecell must be converted to `arrow.b`
    // (not left as `$\downarrow$` which Typst sees as `\d` + `ownarrow`
    // and errors with "unknown variable: ownarrow").
    let src = r"\documentclass{article}
\usepackage{makecell}
\begin{document}
\begin{tabular}{cc}
  Plain $\downarrow$ & \makecell[l]{Agreement $\downarrow$}
\end{tabular}
\end{document}";
    let out = convert_src(src);
    // Both the plain and the makecell-wrapped arrow must be converted
    assert!(
        !out.contains("\\downarrow"),
        "\\downarrow must not appear raw in output; got:\n{}",
        out
    );
    // Both occurrences should be arrow.b
    let count = out.matches("arrow.b").count();
    assert!(
        count >= 2,
        "expected >= 2 arrow.b conversions (plain + makecell); got {} in:\n{}",
        count, out
    );
}

#[test]
fn makecell_without_optional_arg_works() {
    // `\makecell{content}` without optional arg — must still work.
    let src = r"\documentclass{article}
\usepackage{makecell}
\begin{document}
\begin{tabular}{cc}
  A & \makecell{Multi\\ Line}
\end{tabular}
\end{document}";
    let out = convert_src(src);
    assert!(
        out.contains("Multi"),
        "makecell without optional arg must render content; got:\n{}",
        out
    );
}
