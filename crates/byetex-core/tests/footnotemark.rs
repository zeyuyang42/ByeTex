//! `\footnotemark[N]` prints footnote mark N *without* creating a footnote (the
//! body is supplied separately by `\footnotetext`). The emitter used to push a
//! spurious empty `#footnote[]` AND leak the optional `[N]` as escaped literal
//! `\[N\]` text (corpus 2606.12397, author block). It now consumes the optional
//! argument and renders the mark as a superscript.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn footnotemark_with_number_emits_superscript() {
    let t = typ(r"\documentclass{article}\begin{document}Lin\footnotemark[1] x\end{document}");
    assert!(
        t.contains("#super[1]"),
        "optional mark must render as superscript; got:\n{t}"
    );
}

#[test]
fn footnotemark_with_number_does_not_leak_bracket() {
    let t = typ(r"\documentclass{article}\begin{document}Lin\footnotemark[1] x\end{document}");
    assert!(
        !t.contains(r"\[1\]") && !t.contains("[1]\\"),
        "optional [1] must not leak as literal text; got:\n{t}"
    );
}

#[test]
fn footnotemark_with_number_creates_no_empty_footnote() {
    let t = typ(r"\documentclass{article}\begin{document}Lin\footnotemark[1] x\end{document}");
    assert!(
        !t.contains("#footnote[]"),
        "\\footnotemark[N] must not create an empty footnote; got:\n{t}"
    );
}

#[test]
fn footnotemark_in_author_block_is_clean() {
    // The real corpus shape: bold author name + superscript affiliation + mark.
    let t = typ(r"\documentclass{article}\begin{document}\textbf{Yankai Lin\textsuperscript{1}\footnotemark[1]}\end{document}");
    assert!(!t.contains(r"\[1\]"), "no bracket leak in author block; got:\n{t}");
    assert!(t.contains("#super[1]"), "mark rendered; got:\n{t}");
}
