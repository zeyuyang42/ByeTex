//! enumitem `\begin{enumerate}[(a)]` (or `[label=(\roman*)]`) sets the counter
//! FORMAT for the whole list. ByeTex dropped the spec → every styled list fell
//! back to default arabic `+` (corpus: 15 papers). The format should map to a
//! Typst `#enum(numbering: "…")`. Plain lists and pure-option specs keep `+`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn enumerate_alpha_shortcut_style() {
    let t = typ(r"\begin{enumerate}[(a)]\item foo\item bar\end{enumerate}");
    assert!(t.contains("#enum(numbering: \"(a)\""), "got:\n{t}");
    assert!(
        t.contains("[foo]") && t.contains("[bar]"),
        "items lost; got:\n{t}"
    );
}

#[test]
fn enumerate_label_roman_macro_style() {
    let t = typ(r"\begin{enumerate}[label=(\roman*)]\item x\item y\end{enumerate}");
    assert!(t.contains("#enum(numbering: \"(i)\""), "got:\n{t}");
}

#[test]
fn plain_enumerate_keeps_plus() {
    let t = typ(r"\begin{enumerate}\item a\item b\end{enumerate}");
    assert!(t.contains("+ a"), "got:\n{t}");
    assert!(!t.contains("#enum(numbering"), "got:\n{t}");
}

#[test]
fn enumerate_pure_options_keep_plus() {
    // `[noitemsep]` / `[leftmargin=*]` are spacing options, not a counter format.
    let t = typ(r"\begin{enumerate}[noitemsep]\item a\item b\end{enumerate}");
    assert!(t.contains("+ a"), "got:\n{t}");
    assert!(!t.contains("#enum(numbering"), "got:\n{t}");
}
