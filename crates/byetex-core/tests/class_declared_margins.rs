//! Page margins derived from a bundled class's own `\setlength` declarations.
//!
//! LaTeX computes the left margin as `1in + \oddsidemargin` and the text block
//! as `\textwidth`; the right margin is whatever remains. Springer's classes
//! declare both — `llncs.cls` sets `\textwidth{12.2cm}` and
//! `\oddsidemargin{63pt}`, `svmult.cls` sets `117mm`/`63pt` — and we ignored
//! them, emitting the generic 1in margins instead.
//!
//! Measured on the corpus: truth puts those papers' text between 135pt and
//! 131pt (llncs) or 145pt (svmult); we put it at 72pt/66pt, giving
//! `layout_text_width_ratio` up to 1.48 — a text block 48% too wide.
//!
//! The arithmetic reproduces the truth exactly:
//!   llncs   left 72+63 = 135pt, right 612-135-345.8 = 131.2pt  (truth 135/131)
//!   svmult  left 72+63 = 135pt, right 612-135-331.6 = 145.4pt  (truth 135/146)

use byetex_core::{convert, ConvertOptions};
use std::fs;

fn page_line(t: &str) -> String {
    t.lines()
        .find(|l| l.contains("#set page("))
        .unwrap_or("<no page rule>")
        .to_string()
}

fn with_class(name: &str, class_body: &str) -> String {
    let dir = std::env::temp_dir().join(format!("byetex-margins-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("venue.cls"), class_body).unwrap();
    let main = "\\documentclass{venue}\n\\begin{document}\nx\n\\end{document}";
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

#[test]
fn textwidth_and_oddsidemargin_set_the_horizontal_margins() {
    let p = page_line(&with_class(
        "llncs",
        "\\setlength{\\textwidth}{12.2cm}\n\\setlength\\oddsidemargin{63\\p@}\n",
    ));
    assert!(p.contains("left: 135pt"), "1in + 63pt = 135pt; got: {p}");
    assert!(p.contains("right: 131"), "612 - 135 - 345.8 = 131.2pt; got: {p}");
}

#[test]
fn millimetre_declarations_work_too() {
    let p = page_line(&with_class(
        "svmult",
        "\\setlength{\\textwidth}{117mm}\n\\setlength\\oddsidemargin{63\\p@}\n",
    ));
    assert!(p.contains("left: 135pt"), "same left; got: {p}");
    assert!(p.contains("right: 145"), "612 - 135 - 331.6 = 145.4pt; got: {p}");
}

#[test]
fn a_class_declaring_only_textwidth_is_left_alone() {
    // Without `\oddsidemargin` the left edge is unknown, and guessing it would
    // move the text block on classes we currently get right.
    let p = page_line(&with_class("partial", "\\setlength{\\textwidth}{12.2cm}\n"));
    assert!(
        p.contains("margin: (x: 1in, y: 1in)"),
        "an incomplete declaration must not change the margins; got: {p}"
    );
}

#[test]
fn a_class_declaring_nothing_is_unchanged() {
    let p = page_line(&with_class("plain", "\\LoadClass{article}\n"));
    assert!(
        p.contains("margin: (x: 1in, y: 1in)"),
        "default margins preserved; got: {p}"
    );
}
