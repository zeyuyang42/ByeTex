//! `\title*{...}` (svmult/Springer book chapters) is the MAIN title — the `*`
//! only suppresses the running-head/ToC entry (that's `\titlerunning`). ByeTex
//! treated `\title*` as a running-head variant and DROPPED it, losing the title
//! entirely. Harvest it like `\title`. Found by the visual grader on 2605.22312.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn starred_title_is_harvested() {
    let t = typ(r"\documentclass{book}\title*{My Chapter Title}\author{A}\begin{document}\maketitle Body.\end{document}");
    assert!(t.contains("My Chapter Title"), "title* dropped; got:\n{t}");
}
