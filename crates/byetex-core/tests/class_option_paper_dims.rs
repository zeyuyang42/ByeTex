//! Explicit paper DIMENSIONS set by a class option, not a named paper size.
//!
//! `siamart*.cls` ships `\ExecuteOptions{printtrim,...}` as its defaults, and
//! `\DeclareOption{printtrim}{\setlength\paperheight{10in}\setlength\paperwidth
//! {6.75in}}` — so the page is 6.75in x 10in (486x720pt), which is exactly what
//! the LaTeX truth renders for 2605.22281 and 2605.22557 while we emitted
//! us-letter.
//!
//! Resolving this needs the option NAME looked up against its `\DeclareOption`
//! body; the dimensions never appear next to the option list.

use byetex_core::{convert, ConvertOptions};
use std::fs;

fn page_line(t: &str) -> String {
    t.lines()
        .find(|l| l.contains("#set page("))
        .unwrap_or("<no page rule>")
        .to_string()
}

fn with_class(name: &str, class_body: &str, main: &str) -> String {
    let dir = std::env::temp_dir().join(format!("byetex-dims-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("venue.cls"), class_body).unwrap();
    fs::write(dir.join("main.tex"), main).unwrap();
    let out = convert(
        main,
        &ConvertOptions {
            source_name: Some("main.tex".into()),
            base_dir: Some(dir.clone()),
        },
    )
    .typst;
    let _ = fs::remove_dir_all(&dir);
    out
}

const TRIM: &str = "\\DeclareOption{printtrim}\n   {\\setlength\\paperheight {10in}%\n    \\setlength\\paperwidth  {6.75in}}\n\\ExecuteOptions{printtrim,10pt,twoside}\n";
const DOC: &str = "\\documentclass{venue}\n\\begin{document}\nx\n\\end{document}";

#[test]
fn an_executed_default_option_sets_explicit_dimensions() {
    let p = page_line(&with_class("exec", TRIM, DOC));
    assert!(p.contains("width: 6.75in"), "width from the option body; got: {p}");
    assert!(p.contains("height: 10in"), "height from the option body; got: {p}");
}

#[test]
fn an_option_the_document_passes_is_resolved_too() {
    let body = "\\DeclareOption{printtrim}\n   {\\setlength\\paperheight {10in}%\n    \\setlength\\paperwidth  {6.75in}}\n";
    let p = page_line(&with_class(
        "passed",
        body,
        "\\documentclass[printtrim]{venue}\n\\begin{document}\nx\n\\end{document}",
    ));
    assert!(p.contains("width: 6.75in"), "document-passed option; got: {p}");
}

#[test]
fn an_undeclared_option_leaves_the_default_page() {
    let p = page_line(&with_class(
        "undeclared",
        "\\DeclareOption{printtrim}\n   {\\setlength\\paperheight {10in}}\n\\ExecuteOptions{10pt}\n",
        DOC,
    ));
    assert!(
        p.contains("paper: \"us-letter\""),
        "an option that is never executed must not resize; got: {p}"
    );
}

#[test]
fn a_named_paper_option_still_wins_for_the_document() {
    // `\documentclass[a4paper]` is the author's own statement.
    let p = page_line(&with_class(
        "named",
        TRIM,
        "\\documentclass[a4paper]{venue}\n\\begin{document}\nx\n\\end{document}",
    ));
    assert!(p.contains("paper: \"a4\""), "the document's option wins; got: {p}");
}

#[test]
fn a_class_without_dimensions_is_unchanged() {
    let p = page_line(&with_class("plain", "\\LoadClass{article}\n", DOC));
    assert!(p.contains("paper: \"us-letter\""), "default unchanged; got: {p}");
}
