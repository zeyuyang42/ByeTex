//! Task 2b (layout fidelity): two-column body.
//!
//! LaTeX two-column documents (the `twocolumn` class option, or conference
//! classes like IEEEtran / acmart sigconf / ICML) render a full-width title
//! over a two-column body. We mirror that by leaving the generated title block
//! full-width and wrapping the *body* in `#columns(2)[...]`. Single-column
//! documents are unchanged.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// Byte offset of `needle` in `hay`, or `usize::MAX` if absent (so "appears
/// before" comparisons fail loudly rather than panicking on unwrap).
fn pos(hay: &str, needle: &str) -> usize {
    hay.find(needle).unwrap_or(usize::MAX)
}

#[test]
fn twocolumn_option_wraps_body_in_columns() {
    let src = "\\documentclass[twocolumn]{article}\n\
               \\begin{document}\n\\section{Intro}\nBody text.\n\\end{document}";
    let t = typ(src);
    assert!(t.contains("#columns(2)["), "expected a 2-column body wrapper; got:\n{t}");
    // The section heading (body) must sit inside the columns wrapper.
    assert!(
        pos(&t, "#columns(2)[") < pos(&t, "= Intro"),
        "section body should be inside #columns(2)[...]; got:\n{t}"
    );
}

#[test]
fn ieee_class_is_two_column() {
    let src = "\\documentclass[conference]{IEEEtran}\n\\title{T}\n\\author{A}\n\
               \\begin{document}\n\\maketitle\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(t.contains("#columns(2)["), "IEEEtran should be two-column; got:\n{t}");
    // Title block stays full-width: it appears BEFORE the columns wrapper.
    assert!(
        pos(&t, "#align(center)") < pos(&t, "#columns(2)["),
        "title block must be full-width (before the columns); got:\n{t}"
    );
}

#[test]
fn acmart_sigconf_is_two_column() {
    let src = "\\documentclass[sigconf]{acmart}\n\
               \\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(t.contains("#columns(2)["), "acmart sigconf should be two-column; got:\n{t}");
}

#[test]
fn plain_article_stays_single_column() {
    let src = "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        !t.contains("#columns("),
        "plain article must stay single-column; got:\n{t}"
    );
}

#[test]
fn onecolumn_option_overrides_class_default() {
    // An explicit `onecolumn` beats the class's two-column default.
    let src = "\\documentclass[onecolumn]{IEEEtran}\n\
               \\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        !t.contains("#columns("),
        "explicit onecolumn must suppress the columns wrapper; got:\n{t}"
    );
}

#[test]
fn acmart_manuscript_is_single_column() {
    // Journal/manuscript acmart formats are single-column.
    let src = "\\documentclass[manuscript]{acmart}\n\
               \\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        !t.contains("#columns("),
        "acmart manuscript should be single-column; got:\n{t}"
    );
}
