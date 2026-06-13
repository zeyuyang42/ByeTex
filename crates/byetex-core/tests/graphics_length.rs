//! `\includegraphics` length options with LaTeX length units (corpus 2605.31597).
//! `\includegraphics[height=0.4\textheight]{x}` emitted `image(..., height: 0.4\textheight)`,
//! leaking a raw `\` into Typst code → "the character \\ is not valid in code".
//! Fix: `normalize_graphics_length` converts height/paper keywords to a percentage,
//! and drops any still-unconverted `\macro` length to `auto` rather than leaking it.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn textheight_fraction_becomes_percent() {
    let t = typst(r"\includegraphics[height=0.4\textheight]{fig/a.png}");
    assert!(t.contains("height: 40%"), "0.4\\textheight → 40%;\noutput:\n{t}");
    assert!(!t.contains("\\textheight"), "must not leak \\textheight;\noutput:\n{t}");
}

#[test]
fn unknown_length_macro_drops_to_auto_not_backslash() {
    let t = typst(r"\includegraphics[width=0.3\foobar]{fig/a.png}");
    // The key invariant: no raw LaTeX backslash leaks into the image() call.
    let img_line: String = t.lines().filter(|l| l.contains("image(")).collect();
    assert!(
        !img_line.contains('\\'),
        "no backslash may leak into image() args;\noutput:\n{t}"
    );
}

#[test]
fn known_units_still_pass_through() {
    let t = typst(r"\includegraphics[width=3cm]{fig/a.png}");
    assert!(t.contains("width: 3cm"), "explicit units pass through;\noutput:\n{t}");
}
