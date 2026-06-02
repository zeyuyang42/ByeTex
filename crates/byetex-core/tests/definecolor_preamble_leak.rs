//! Bug (corpus 2605.22779/22507/22817): `\definecolor{name}{model}{spec}`
//! preamble declarations leaked into the body verbatim — a block of
//! `\definecolor{cDeepBlue}{HTML}{1A5276}` lines rendered as literal text right
//! next to the abstract. `\definecolor` defines a colour alias with no visible
//! body (like the already-dropped `\colorlet`); it must be dropped, args and
//! all.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn definecolor_is_dropped_not_leaked() {
    let src = "\\documentclass{article}\n\
        \\definecolor{cDeepBlue}{HTML}{1A5276}\n\
        \\definecolor{mgreen}{rgb}{0.0,0.5,0.0}\n\
        \\begin{document}\nBody text here.\n\\end{document}\n";
    let t = typ(src);
    // Neither the command nor its leaked args may appear in the output.
    assert!(
        !t.contains("definecolor"),
        "\\definecolor must be dropped, not leaked; got:\n{t}"
    );
    assert!(
        !t.contains("1A5276") && !t.contains("cDeepBlue") && !t.contains("HTML"),
        "the \\definecolor arguments must not leak; got:\n{t}"
    );
    // The real body must survive.
    assert!(t.contains("Body text here"), "body must remain; got:\n{t}");
}
