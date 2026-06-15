//! `\resizebox{width}{height}{content}` (graphicx) wraps content scaled to a
//! target box. ByeTex had no handler, so the whole node — INCLUDING the wrapped
//! tabular/figure — was dropped (corpus: 21 papers, `\resizebox{\textwidth}{!}{…}`
//! is the standard idiom for fitting a wide table to the text width). The wrapped
//! content must survive; the scale-to-fit sizing is secondary.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn resizebox_preserves_wrapped_tabular() {
    let t = typ(r"\resizebox{\textwidth}{!}{\begin{tabular}{cc}ALPHA & BETA \\ \end{tabular}}");
    assert!(
        t.contains("ALPHA") && t.contains("BETA"),
        "wrapped table content must survive; got:\n{t}"
    );
}

#[test]
fn resizebox_preserves_plain_content() {
    let t = typ(r"x \resizebox{5cm}{!}{KEEPME} y");
    assert!(
        t.contains("KEEPME"),
        "wrapped content must survive; got:\n{t}"
    );
    assert!(
        t.contains('x') && t.contains('y'),
        "surrounding text preserved; got:\n{t}"
    );
}
