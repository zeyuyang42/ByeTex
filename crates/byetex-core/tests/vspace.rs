//! `\vspace{len}` / `\hspace{len}` were dropped (audit: 39 papers). When the
//! length is a plain Typst dimension (em/cm/mm/in/pt/ex, optionally signed) it
//! now becomes `#v(len)` / `#h(len)`. LaTeX length macros (`\baselineskip`,
//! `\dimexpr…`) have no Typst analog, so those keep being dropped (never a
//! broken compile). `\smallskip`/`\medskip`/`\bigskip` were already handled.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn vspace_emits_v() {
    assert!(typ(r"A\vspace{1em}B").contains("#v(1em)"));
}

#[test]
fn vspace_negative_length() {
    assert!(typ(r"A\vspace{-0.5cm}B").contains("#v(-0.5cm)"));
}

#[test]
fn hspace_emits_h() {
    assert!(typ(r"A\hspace{2em}B").contains("#h(2em)"));
}

#[test]
fn vspace_macro_length_is_dropped() {
    let t = typ(r"A\vspace{\baselineskip}B");
    assert!(!t.contains("#v("), "macro length must drop, not emit a broken #v; got:\n{t}");
    assert!(t.contains('A') && t.contains('B'), "surrounding text kept; got:\n{t}");
}
