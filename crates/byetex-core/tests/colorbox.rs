//! Colour follow-ups to the `\textcolor` fix:
//! - `\colorbox{bg}{x}`  → `#highlight(fill: bg)[x]`
//! - `\fcolorbox{fr}{bg}{x}` → `#box(fill: bg, stroke: fr, inset: 2pt)[x]`
//!   (was a generic command that DROPPED its content entirely)
//! - `\textcolor` in MATH now colours too: `#text(fill: c)[$…$]`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn colorbox_becomes_highlight() {
    let t = typ(r"\colorbox{yellow}{important}");
    assert!(
        t.contains("#highlight(fill: yellow)[important]"),
        "got:\n{t}"
    );
}

#[test]
fn fcolorbox_keeps_content_in_a_box() {
    let t = typ(r"\fcolorbox{red}{yellow}{boxed}");
    assert!(
        t.contains("boxed"),
        "content must not be dropped; got:\n{t}"
    );
    assert!(
        t.contains("#box(fill: yellow, stroke: red"),
        "frame+bg box not emitted; got:\n{t}"
    );
}

#[test]
fn math_textcolor_applies_fill() {
    let t = typ(r"$\textcolor{red}{x}$");
    assert!(
        t.contains("#text(fill: red)[$"),
        "math colour not applied; got:\n{t}"
    );
}

#[test]
fn textcolor_still_works() {
    let t = typ(r"\textcolor{blue}{hi}");
    assert!(t.contains("#text(fill: blue)[hi]"), "got:\n{t}");
}
