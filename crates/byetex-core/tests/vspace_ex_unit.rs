//! `ex` is not a Typst length unit. ByeTex listed it as 1:1-convertible, so
//! `\vspace{1ex}` emitted `#v(1ex)` and Typst read `1e` as broken scientific
//! notation ("invalid floating point number: 1e"; corpus 2605.31603, a false
//! known_pass). Fix: approximate 1ex ≈ 0.5em.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn vspace_ex_converts_to_em() {
    let t = typst(r"x\vspace{1ex}y");
    assert!(t.contains("#v(0.5em)"), "1ex → 0.5em;\noutput:\n{t}");
    assert!(!t.contains("1ex"), "must not emit the invalid `ex` unit;\noutput:\n{t}");
}

#[test]
fn vspace_em_pt_still_pass_through() {
    assert!(typst(r"x\vspace{2em}y").contains("#v(2em)"));
    assert!(typst(r"x\vspace{3pt}y").contains("#v(3pt)"));
}
