//! Page TOP/BOTTOM margins from a class's own vertical `\setlength` block.
//!
//! The vertical counterpart of `class_declared_margins.rs`. LaTeX puts the top
//! of the text block at
//!
//!   1in + \voffset + \topmargin + \headheight + \headsep
//!
//! and makes it `\textheight` tall; the bottom margin is the remainder. We
//! ignored all five and emitted a flat `y: 1in`.
//!
//! Verified against truth on 2605.22557 (siamart, 6.75in x 10in = 720pt tall),
//! whose class declares voffset -.5in, topmargin \z@, headheight 10pt,
//! headsep .2in, textheight 598pt:
//!
//!   top    = 72 - 36 + 0 + 10 + 14.4 = 60.4pt   (truth body starts at 62.5)
//!   bottom = 720 - 60.4 - 598        = 61.6pt   (truth measures 61.7)
//!
//! Note the paper's `layout_top_margin_ratio` of 1.859 is NOT all body offset:
//! truth's topmost ink there is a running header we do not emit. The body
//! placement is the part this fixes.

use byetex_core::{convert, ConvertOptions};
use std::fs;

fn page_line(t: &str) -> String {
    t.lines()
        .find(|l| l.contains("#set page("))
        .unwrap_or("<no page rule>")
        .to_string()
}

fn with_class(name: &str, class_body: &str) -> String {
    let dir = std::env::temp_dir().join(format!("byetex-vmargin-{}-{}", name, std::process::id()));
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

/// The full five-length stack, on us-letter (792pt tall).
///   top    = 72 - 36 + 0 + 10 + 14.4 = 60.4
///   bottom = 792 - 60.4 - 598        = 133.6
#[test]
fn vertical_lengths_set_the_top_and_bottom_margins() {
    let p = page_line(&with_class(
        "siam",
        "\\setlength\\voffset{-.5in}\n\\setlength\\topmargin{\\z@}\n\\setlength\\headheight{10\\p@}\n\\setlength\\headsep{.2in}\n\\setlength{\\textheight}{598pt}\n",
    ));
    assert!(p.contains("top: 60.4pt"), "72-36+0+10+14.4 = 60.4pt; got: {p}");
    assert!(p.contains("bottom: 133.6pt"), "792-60.4-598 = 133.6pt; got: {p}");
}

/// `\z@` is TeX's own name for 0pt and must parse as a length, not be skipped —
/// `\topmargin{\z@}` is how siamart states a zero top margin.
#[test]
fn zero_length_macro_parses() {
    let p = page_line(&with_class(
        "zed",
        "\\setlength\\topmargin{\\z@}\n\\setlength{\\textheight}{600pt}\n",
    ));
    assert!(p.contains("top: 72pt"), "1in + 0 = 72pt; got: {p}");
    assert!(p.contains("bottom: 120pt"), "792-72-600 = 120pt; got: {p}");
}

/// Both axes declared: horizontal and vertical must compose in one page rule.
#[test]
fn horizontal_and_vertical_compose() {
    let p = page_line(&with_class(
        "both",
        "\\setlength{\\textwidth}{12.2cm}\n\\setlength\\oddsidemargin{63\\p@}\n\\setlength\\topmargin{\\z@}\n\\setlength{\\textheight}{600pt}\n",
    ));
    assert!(p.contains("left: 135pt"), "got: {p}");
    assert!(p.contains("top: 72pt"), "got: {p}");
}

/// A text block taller than the page is a misread; keep the neutral default.
#[test]
fn overtall_declaration_is_declined() {
    let p = page_line(&with_class(
        "tall",
        "\\setlength\\topmargin{\\z@}\n\\setlength{\\textheight}{2000pt}\n",
    ));
    assert!(!p.contains("bottom: -"), "no negative margin; got: {p}");
    assert!(p.contains("y: 1in"), "neutral vertical default; got: {p}");
}

/// `\textheight` alone leaves the top edge unknown; declining is correct.
#[test]
fn textheight_without_topmargin_declines() {
    let p = page_line(&with_class(
        "partial",
        "\\setlength{\\textheight}{600pt}\n",
    ));
    assert!(p.contains("y: 1in"), "under-determined; got: {p}");
}
