//! Page margins from a `geometry` KEY-VALUE block, not `\setlength`.
//!
//! `class_declared_margins.rs` covers classes that state their text block as
//! `\setlength{\textwidth}` + `\setlength{\oddsidemargin}`. A second, equally
//! common form states it as geometry keys — and we ignored those entirely,
//! falling back to the neutral 1in margins.
//!
//! `neurips_2026.sty` is the corpus's case: it loads geometry with only a paper
//! option and then, inside `\AtBeginDocument`, issues
//!
//!   \newgeometry{textheight=9in, textwidth=5.5in, top=1in, ...}
//!
//! Measured on 2605.22814/22549/22794/22507, truth puts the text block at
//! left 108pt / width 396pt / right 108pt — exactly `textwidth=5.5in` centred on
//! letter (612 - 396 = 216, halved). We emitted 72pt/472pt/68pt: a text block
//! 19% too wide, and `layout_right_margin_ratio` 0.64.

use byetex_core::{convert, ConvertOptions};
use std::fs;

fn page_line(t: &str) -> String {
    t.lines()
        .find(|l| l.contains("#set page("))
        .unwrap_or("<no page rule>")
        .to_string()
}

fn with_class(name: &str, class_body: &str) -> String {
    let dir =
        std::env::temp_dir().join(format!("byetex-geomkey-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("venue.sty"), class_body).unwrap();
    let main = "\\documentclass{article}\n\\usepackage{venue}\n\\begin{document}\nx\n\\end{document}";
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

/// The corpus case, verbatim: a bare `textwidth` centres the block.
#[test]
fn newgeometry_textwidth_centres_the_text_block() {
    let p = page_line(&with_class(
        "neurips",
        "\\usepackage[verbose=true,letterpaper]{geometry}\n\\AtBeginDocument{\n  \\newgeometry{\n    textheight=9in,\n    textwidth=5.5in,\n    top=1in,\n    headheight=12pt\n  }\n}\n",
    ));
    assert!(p.contains("left: 108pt"), "(612 - 396)/2 = 108pt; got: {p}");
    assert!(p.contains("right: 108pt"), "(612 - 396)/2 = 108pt; got: {p}");
}

/// `left` pins the block; `right` is then the remainder, not a second centring.
#[test]
fn textwidth_with_left_derives_the_right_margin() {
    let p = page_line(&with_class(
        "pinned",
        "\\geometry{textwidth=5in, left=1in}\n",
    ));
    assert!(p.contains("left: 72pt"), "left=1in; got: {p}");
    assert!(p.contains("right: 180pt"), "612 - 72 - 360 = 180pt; got: {p}");
}

/// A symmetric `margin=` needs no width at all.
#[test]
fn symmetric_margin_key() {
    let p = page_line(&with_class("sym", "\\geometry{margin=1.25in}\n"));
    assert!(p.contains("left: 90pt"), "1.25in = 90pt; got: {p}");
    assert!(p.contains("right: 90pt"), "1.25in = 90pt; got: {p}");
}

/// A block that cannot fit the page is a misread; keep the neutral default
/// rather than emitting a negative margin.
#[test]
fn overwide_declaration_is_declined() {
    let p = page_line(&with_class("bad", "\\geometry{textwidth=20in, left=1in}\n"));
    assert!(!p.contains("left: 72pt, right: -"), "no negative margin: {p}");
    assert!(p.contains("1in") || p.contains("72pt"), "neutral default: {p}");
}

/// A package loaded by PATH (`\usepackage{style/neurips_2026}`) must be scanned
/// too. Corpus 2605.22507 keeps the same NeurIPS style in a `style/`
/// subdirectory; the project scan only read `.sty` files sitting at the top
/// level, so that paper alone kept the neutral 1in while its three siblings
/// picked up the declared 5.5in block.
#[test]
fn package_loaded_by_subdirectory_path_is_scanned() {
    let dir = std::env::temp_dir().join(format!("byetex-geomkey-subdir-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("style")).unwrap();
    fs::write(
        dir.join("style").join("venue.sty"),
        "\\newgeometry{textwidth=5.5in, top=1in}\n",
    )
    .unwrap();
    let main =
        "\\documentclass{article}\n\\usepackage[main]{style/venue}\n\\begin{document}\nx\n\\end{document}";
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
    let p = page_line(&out);
    assert!(p.contains("left: 108pt"), "(612 - 396)/2 = 108pt; got: {p}");
    assert!(p.contains("right: 108pt"), "(612 - 396)/2 = 108pt; got: {p}");
}

/// A geometry block that declares its OWN page size describes a different page
/// than the one we are emitting, so its margins cannot be transplanted onto ours.
///
/// Corpus 2605.31597/31598 (Springer llncs + `eccv.sty`) is the case. `eccv.sty`
/// carries, inside a conditional branch that the truth build does not take:
///
///   \RequirePackage[width=122mm,left=12mm,paperwidth=146mm,...]{geometry}
///
/// Reading `left=12mm` against our 612pt letter page put the text block at 34pt
/// where truth has 135pt — `layout_left_margin_ratio` 1.001 -> 0.252. The
/// `\setlength` values in `llncs.cls` are the ones actually in force, and are
/// reached only if the mismatched block declines.
#[test]
fn geometry_block_for_a_different_paper_size_is_declined() {
    let dir = std::env::temp_dir().join(format!("byetex-geomkey-paperw-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("venue.sty"),
        "\\RequirePackage[width=122mm,left=12mm,paperwidth=146mm,height=193mm,top=12mm,paperheight=217mm]{geometry}\n",
    )
    .unwrap();
    fs::write(
        dir.join("book.cls"),
        "\\setlength{\\textwidth}{12.2cm}\n\\setlength\\oddsidemargin{63\\p@}\n",
    )
    .unwrap();
    let main = "\\documentclass{book}\n\\usepackage{venue}\n\\begin{document}\nx\n\\end{document}";
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
    let p = page_line(&out);
    assert!(
        !p.contains("left: 34"),
        "12mm belongs to a 146mm-wide page, not ours; got: {p}"
    );
    assert!(
        p.contains("left: 135pt"),
        "the in-force \\setlength must win: 1in + 63pt = 135pt; got: {p}"
    );
}
