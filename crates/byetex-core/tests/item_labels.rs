//! A custom `\item[(a)]` label in an itemize/enumerate replaces the auto marker
//! in LaTeX. ByeTex emitted the auto Typst marker AND leaked the bracket as an
//! escaped literal — `+ \[(a)\] foo` (wrong number + garbage). The label should
//! instead become a Typst term item (`/ (a): foo`), which is exactly the
//! description-list mechanism `\item[..]` uses. Plain lists keep `+`/`-`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn custom_item_label_renders_as_term_not_broken_marker() {
    let t = typ(r"\begin{enumerate}\item[(a)] foo \item[(b)] bar\end{enumerate}");
    assert!(!t.contains(r"\[(a)\]"), "bracket label leaked as escaped literal; got:\n{t}");
    assert!(t.contains("/ (a): foo"), "label (a) not preserved as a term; got:\n{t}");
    assert!(t.contains("/ (b): bar"), "label (b) not preserved as a term; got:\n{t}");
}

#[test]
fn plain_enumerate_still_uses_plus() {
    let t = typ(r"\begin{enumerate}\item foo\item bar\end{enumerate}");
    assert!(t.contains("+ foo") && t.contains("+ bar"), "got:\n{t}");
}

#[test]
fn plain_itemize_still_uses_dash() {
    let t = typ(r"\begin{itemize}\item foo\item bar\end{itemize}");
    assert!(t.contains("- foo") && t.contains("- bar"), "got:\n{t}");
}
