//! biblatex bibliography support + `\nocite` (corpus 2605.30843, 2605.31009).
//!
//! Both papers use biblatex (`\addbibresource` + `\printbibliography`), which
//! ByeTex dropped — so `\cite{k}` → `@k` dangled with no `#bibliography` (119
//! "label does not exist" errors in 31009). And `\nocite{*}` emitted
//! `[cite: missing key `*`]`, whose lone `*` is an unclosed bold marker (30843).
//!
//! Fix: collect `\addbibresource` paths and render `#bibliography(...)` from
//! `\printbibliography`; drop `\nocite` (it prints nothing).

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn printbibliography_renders_addbibresource() {
    let src = "\\addbibresource{refs.bib}\n\
        Text~\\cite{Smith2020}.\n\
        \\printbibliography";
    let t = typst(src);
    assert!(
        t.contains("#bibliography(\"refs.bib\")"),
        "biblatex \\printbibliography must render #bibliography from \\addbibresource;\noutput:\n{t}"
    );
    assert!(
        t.contains("@Smith2020"),
        "the cite must reference the rendered bib;\noutput:\n{t}"
    );
}

#[test]
fn printbibliography_option_does_not_leak() {
    let src = "\\addbibresource{refs.bib}\n\\printbibliography[title={References}]";
    let t = typst(src);
    assert!(
        !t.contains("title={References}") && !t.contains("[title="),
        "the [title={{...}}] option must not leak into the body;\noutput:\n{t}"
    );
}

#[test]
fn nocite_star_emits_nothing() {
    let src = "\\addbibresource{refs.bib}\nBody.\n\\nocite{*}\n\\printbibliography";
    let t = typst(src);
    assert!(
        !t.contains("cite: missing key") && !t.contains("`*`"),
        "\\nocite{{*}} must not emit a placeholder (lone `*` is unclosed bold);\noutput:\n{t}"
    );
}
