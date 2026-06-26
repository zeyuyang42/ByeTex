//! REVTeX/apsrev `.bbl` files wrap every reference field in custom macros
//! (`\bibinfo{field}{value}`, `\bibnamefont{X}`, `\BibitemShut{NoStop}`, …).
//! ByeTex dropped the wrapped values (authors/journals vanished) and leaked the
//! structural markers as literal text ("bibitemNoStop"). Unwrap the value macros
//! and drop the markers. Found by the visual grader on 2605.31203 (APS physics).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn bibitem_value_macros_unwrapped() {
    let t = typ(r"\bibnamefont{Smith} in \bibinfo{journal}{PRB} \bibfield{volume}{106} \BibitemShut{NoStop}");
    assert!(t.contains("Smith"), "bibnamefont content lost; got:\n{t}");
    assert!(t.contains("PRB"), "bibinfo value lost; got:\n{t}");
    assert!(t.contains("106"), "bibfield value lost; got:\n{t}");
}

#[test]
fn bibitem_markers_dropped() {
    let t = typ(r"text \BibitemShut{NoStop} \BibitemOpen more");
    assert!(!t.contains("NoStop") && !t.contains("BibitemShut") && !t.contains("BibitemOpen"),
        "structural markers leaked; got:\n{t}");
}
