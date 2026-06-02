//! Design fidelity (user visual review): the abstract rendered as a grey
//! rounded box — `#block(width: 90%, radius: 4pt, fill: luma(245))[*Abstract.*
//! …]` — a "neutral template" look nothing like LaTeX. A standard `article`
//! abstract is a centered bold "Abstract" heading above a narrowed, justified
//! text block, with no fill/border/rounded corners. Match that.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DOC: &str = "\\documentclass{article}\n\\title{T}\\author{A}\n\
    \\begin{document}\\maketitle\n\
    \\begin{abstract}\nWe study a thing and report results.\n\\end{abstract}\n\
    Body.\n\\end{document}\n";

#[test]
fn abstract_is_a_centered_heading_not_a_grey_box() {
    let t = typ(DOC);
    // The old grey-rounded-box styling must be gone.
    assert!(
        !t.contains("fill: luma(245)") && !t.contains("radius: 4pt"),
        "abstract must not be a filled rounded box; got:\n{t}"
    );
    // No inline `*Abstract.*` lead-in.
    assert!(
        !t.contains("*Abstract.*"),
        "abstract label must be a centered heading, not an inline lead-in; got:\n{t}"
    );
    // A centered, bold "Abstract" heading.
    assert!(
        t.contains("#align(center)[#text(weight: \"bold\")[Abstract]]"),
        "expected a centered bold Abstract heading; got:\n{t}"
    );
    // The body text survives.
    assert!(
        t.contains("We study a thing and report results"),
        "abstract text must be preserved; got:\n{t}"
    );
}

#[test]
fn abstract_text_is_in_a_narrowed_block() {
    let t = typ(DOC);
    // Indented both sides → a narrower column, like LaTeX's abstract.
    assert!(
        t.contains("#pad(x:"),
        "abstract text should sit in a horizontally-padded (narrower) block; got:\n{t}"
    );
}
