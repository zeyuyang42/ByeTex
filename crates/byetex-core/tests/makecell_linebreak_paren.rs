//! Inside `\makecell`, `\\` becomes `#linebreak()`. When the source has `\\(`
//! (a parenthesized group right after the break), the output `#linebreak()(…)`
//! is parsed by Typst as a call chain → "expected function, found content"
//! (corpus 2605.31063 `\makecell{\textbf{Estimates}\\($\Delta F$)}`). Fix: a
//! zero-width space after `#linebreak()` when `(` immediately follows.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn makecell_linebreak_then_paren_does_not_chain() {
    let t = typst(r"\begin{tabular}{c}\makecell{Estimates\\($x$)} \\\end{tabular}");
    assert!(
        !t.contains("#linebreak()("),
        "#linebreak() must not be glued to `(`;\noutput:\n{t:?}"
    );
    assert!(
        t.contains("#linebreak()\u{200B}("),
        "expected a zero-width-space break between #linebreak() and `(`;\noutput:\n{t:?}"
    );
}
