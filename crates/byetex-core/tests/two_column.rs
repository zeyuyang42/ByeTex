//! Layout fidelity: two-column body.
//!
//! LaTeX two-column documents (the `twocolumn` class option, or conference
//! classes like IEEEtran / acmart sigconf / ICML / ACL) render a full-width
//! title over a two-column body. We mirror that with a PAGE-level
//! `#set page(..., columns: 2)` and a full-width spanning title float
//! (`#place(top + center, scope: "parent", float: true)[...]`). This replaced
//! the old `#columns(2)[body]` content-block wrap, which blew up on figure-heavy
//! docs (corpus 2605.31586: 81 pages). Single-column documents are unchanged.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn pos(hay: &str, needle: &str) -> usize {
    hay.find(needle).unwrap_or(usize::MAX)
}

const PAGE_2COL: &str = ", columns: 2)";
const SPAN_TITLE: &str = "#place(top + center, scope: \"parent\", float: true)[";

#[test]
fn twocolumn_option_sets_page_columns() {
    let src = "\\documentclass[twocolumn]{article}\n\
               \\begin{document}\n\\section{Intro}\nBody text.\n\\end{document}";
    let t = typ(src);
    assert!(t.contains(PAGE_2COL), "expected page columns: 2; got:\n{t}");
    assert!(
        !t.contains("#columns(2)["),
        "must not use the old content-block wrap; got:\n{t}"
    );
}

#[test]
fn ieee_class_is_two_column() {
    let src = "\\documentclass[conference]{IEEEtran}\n\\title{T}\n\\author{A}\n\
               \\begin{document}\n\\maketitle\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        t.contains(PAGE_2COL),
        "IEEEtran should be two-column; got:\n{t}"
    );
    // Title spans both columns via the parent-scoped float.
    assert!(
        t.contains(SPAN_TITLE),
        "IEEE title must be a full-width spanning float; got:\n{t}"
    );
    assert!(
        pos(&t, SPAN_TITLE) < pos(&t, "= Intro"),
        "title float must come before the body; got:\n{t}"
    );
}

#[test]
fn acmart_sigconf_is_two_column() {
    let src = "\\documentclass[sigconf]{acmart}\n\
               \\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        t.contains(PAGE_2COL),
        "acmart sigconf should be two-column; got:\n{t}"
    );
}

#[test]
fn plain_article_stays_single_column() {
    let src =
        "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        !t.contains(PAGE_2COL),
        "plain article must stay single-column; got:\n{t}"
    );
}

#[test]
fn onecolumn_option_overrides_class_default() {
    let src = "\\documentclass[onecolumn]{IEEEtran}\n\
               \\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        !t.contains(PAGE_2COL),
        "explicit onecolumn must suppress page columns; got:\n{t}"
    );
}

#[test]
fn acmart_manuscript_is_single_column() {
    let src = "\\documentclass[manuscript]{acmart}\n\
               \\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
    let t = typ(src);
    assert!(
        !t.contains(PAGE_2COL),
        "acmart manuscript should be single-column; got:\n{t}"
    );
}
