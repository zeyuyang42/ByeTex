//! Two-column class detection + page-level rendering (fix for the page-overflow
//! cluster). Two gaps in detection:
//! - ACL (`\usepackage{acl}` on plain `article`) had no DocClass → single-column.
//! - IEEE Transactions variants (`\documentclass{IEEEtranTCOM}`) matched only the
//!   exact `IEEEtran` name → single-column.
//!
//! Both are two-column. Document-level two-column now uses a PAGE-level
//! `#set page(..., columns: 2,` with a full-width spanning title float
//! (`#place(..., scope: "parent", float: true)`), not a `#columns(2)[body]` wrap
//! (which blew up on figure-heavy docs — corpus 2605.31586: 81 pages).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const PAGE_2COL: &str = ", columns: 2,";
const SPAN_TITLE: &str = "#place(top + center, scope: \"parent\", float: true)[";

#[test]
fn acl_renders_page_two_column_with_spanning_title() {
    let src = "\\documentclass{article}\n\\usepackage{acl}\n\\title{T}\n\
        \\begin{document}\\maketitle\n\\begin{abstract}A.\\end{abstract}\n\
        \\section{Intro}Body.\\end{document}";
    let t = typ(src);
    assert!(
        t.contains(PAGE_2COL),
        "ACL must set page columns: 2;\noutput:\n{t}"
    );
    assert!(
        t.contains(SPAN_TITLE),
        "ACL title must span both columns;\noutput:\n{t}"
    );
    assert!(
        !t.contains("#columns(2)["),
        "must not use the old content-block wrap;\noutput:\n{t}"
    );
    assert!(
        t.contains("Abstract"),
        "abstract must still render;\noutput:\n{t}"
    );
}

#[test]
fn ieeetran_variant_renders_page_two_column() {
    let src = "\\documentclass[journal,10pt]{IEEEtranTCOM}\n\\title{T}\n\
        \\begin{document}\\maketitle\n\\section{Intro}Body.\\end{document}";
    let t = typ(src);
    assert!(
        t.contains(PAGE_2COL),
        "IEEEtranTCOM must set page columns: 2;\noutput:\n{t}"
    );
    assert!(
        t.contains(SPAN_TITLE),
        "IEEE title must span both columns;\noutput:\n{t}"
    );
}

#[test]
fn plain_article_stays_single_column() {
    let src = "\\documentclass{article}\n\\title{T}\n\
        \\begin{document}\\maketitle\n\\section{Intro}Body.\\end{document}";
    let t = typ(src);
    assert!(
        !t.contains(PAGE_2COL),
        "plain article must stay single-column;\noutput:\n{t}"
    );
    assert!(
        !t.contains(SPAN_TITLE),
        "single-column title is not a spanning float;\noutput:\n{t}"
    );
}
