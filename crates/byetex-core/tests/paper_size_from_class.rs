//! Paper size declared by a BUNDLED class rather than by `\documentclass`.
//!
//! `a4paper` in the document's own option list was already honoured. Two corpus
//! papers set it inside the class they ship — `\LoadClass[a4paper]{article}` in
//! `iopjournal.cls`, and an `a4paper` geometry option in a thesis class — so we
//! emitted us-letter (612x792) where the LaTeX truth renders A4 (595x842).
//!
//! Page size is a first-order layout property: every margin, column and
//! lines-per-page comparison is measured against it.

use byetex_core::{convert, ConvertOptions};
use std::fs;

fn paper_line(t: &str) -> String {
    t.lines()
        .find(|l| l.contains("#set page("))
        .unwrap_or("<no page rule>")
        .to_string()
}

fn convert_with_class(name: &str, class_body: &str, main: &str) -> String {
    // Per-TEST directory: cargo runs these concurrently and they all write a
    // `venue.cls` that the emitter reads back, so a shared path has them picking
    // up each other's class file.
    let dir = std::env::temp_dir().join(format!("byetex-paper-{}-{}", name, std::process::id()));
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

const DOC: &str = "\\documentclass{venue}\n\\begin{document}\nx\n\\end{document}";

#[test]
fn load_class_a4paper_sets_the_page_size() {
    let p = paper_line(&convert_with_class(
        "loadclass",
        "\\LoadClass[a4paper]{article}\n",
        DOC,
    ));
    assert!(p.contains("paper: \"a4\""), "\\LoadClass[a4paper]; got: {p}");
}

#[test]
fn a_geometry_a4paper_option_in_the_class_counts() {
    let p = paper_line(&convert_with_class(
        "geometry",
        "\\usepackage[a4paper,margin=2.5cm]{geometry}\n",
        DOC,
    ));
    assert!(p.contains("paper: \"a4\""), "geometry a4paper; got: {p}");
}

#[test]
fn the_documents_own_option_still_wins() {
    // An explicit `\documentclass[letterpaper]` outranks the class default.
    let p = paper_line(&convert_with_class(
        "docwins",
        "\\LoadClass[a4paper]{article}\n",
        "\\documentclass[letterpaper]{venue}\n\\begin{document}\nx\n\\end{document}",
    ));
    assert!(
        p.contains("paper: \"us-letter\""),
        "the document's own option wins; got: {p}"
    );
}

#[test]
fn a_bare_mention_does_not_change_the_paper() {
    // The word must appear in an OPTION list, not in prose or a comment —
    // otherwise a class that merely documents `a4paper` would resize the page.
    let p = paper_line(&convert_with_class(
        "comment",
        "% supports a4paper if you want it\n\\LoadClass{article}\n",
        DOC,
    ));
    assert!(
        p.contains("paper: \"us-letter\""),
        "a comment must not set the paper; got: {p}"
    );
}

#[test]
fn no_declaration_stays_us_letter() {
    let p = paper_line(&convert_with_class("none", "\\LoadClass{article}\n", DOC));
    assert!(p.contains("paper: \"us-letter\""), "default unchanged; got: {p}");
}

#[test]
fn a_geometry_command_option_counts_too() {
    // `\geometry{a4paper, ...}` is the command form of the same declaration.
    // gh-dzwaneveld-tudelft-thesis uses it and its truth renders A4 (595x842).
    let p = paper_line(&convert_with_class(
        "geomcmd",
        "\\LoadClass{book}\n\\geometry{a4paper,hscale=0.75,vscale=0.8}\n",
        DOC,
    ));
    assert!(p.contains("paper: \"a4\""), "\\geometry{{a4paper}}; got: {p}");
}
