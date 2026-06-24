//! Regression: a "wrapper" `\newcommand` that defines another `\newcommand`
//! (`\newcommand{\mytok}[2]{\newcommand{#1}{{\color{\colourtok}#2}}}`) must not
//! leak its inner definition body when called. The prepass
//! (`harvest_wrapper_newcommands`) already registers the inner macro
//! (`\foo` → `{\color{black}bar}`), so the call site `\mytok{\foo}{bar}` is a
//! definition that produces no document output — it must emit nothing, not
//! splice `{\color{\colourtok}#2}` (→ `black#2`) into the body (dogfood backlog
//! H3 colour-residue, 2605.22821).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn wrapper_newcommand_call_emits_nothing_but_defines_inner() {
    let src = r"\documentclass{article}
\newcommand{\colourtok}{black}
\newcommand{\mytok}[2]{\newcommand{#1}{{\color{\colourtok}#2}}}
\begin{document}
\mytok{\foo}{bar}
Here is \foo.
\end{document}";
    let t = typ(src);
    // The inner macro still works: \foo → black-coloured "bar".
    assert!(t.contains("bar"), "inner macro `\\foo` lost its content; got:\n{t}");
    // The call site must NOT leak the inner definition body.
    assert!(
        !t.contains("#2") && !t.contains(r"\#2"),
        "wrapper call leaked an unsubstituted `#2`; got:\n{t}"
    );
    // The colour-name string must not leak as standalone body text. `\foo`'s
    // own expansion renders "bar" via `text(fill: ...)`, not the bare word
    // "black" followed by "#2".
    assert!(
        !t.contains("black\\#2") && !t.contains("black#2"),
        "wrapper call leaked `black#2`; got:\n{t}"
    );
}

#[test]
fn ordinary_newcommand_still_registers_and_expands() {
    // A non-wrapper macro must be unaffected.
    let src = r"\documentclass{article}
\newcommand{\greet}[1]{Hello #1!}
\begin{document}
\greet{world}
\end{document}";
    let t = typ(src);
    assert!(t.contains("Hello"), "ordinary macro broke; got:\n{t}");
    assert!(t.contains("world"), "ordinary macro arg lost; got:\n{t}");
}
