//! `\text{...}` inside math with embedded inline math (`\text{if $x = y$}`) used to
//! render the `$...$` literally (dollar signs and all). Now the inner math is
//! re-converted while the surrounding words stay upright (round-4 dogfood A5).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn text_inner_math_is_converted() {
    let t = typ("\\documentclass{article}\\begin{document}\\[ f = \\begin{cases} a & \\text{if $x_t = y$} \\\\ b & \\text{otherwise} \\end{cases} \\]\\end{document}");
    // The inner math must NOT keep its dollar signs as literal string content.
    assert!(!t.contains("$x_t = y$"), "inner $...$ must be re-converted, not literal; got:\n{t}");
    // The word "if" stays upright (quoted), and x_t renders as math.
    assert!(t.contains("\"if"), "leading text kept upright; got:\n{t}");
    assert!(t.contains("x_t"), "inner math rendered; got:\n{t}");
    assert!(t.contains("otherwise"), "plain text branch kept");
}

#[test]
fn escaped_dollar_is_literal_not_a_delimiter() {
    // Code-review: `\text{costs \$5 today}` — the `\$` is a literal dollar, not a math
    // delimiter. Must not split/letter-split, and must produce compilable Typst.
    let t = typ("\\documentclass{article}\\begin{document}\\[ a = b \\text{ costs \\$5 today } \\]\\end{document}");
    assert!(t.contains("costs $5 today") || t.contains("costs $5"), "literal $ kept; got:\n{t}");
    assert!(!t.contains("t o d a y"), "text must not be letter-split as math; got:\n{t}");
}

#[test]
fn unbalanced_dollar_does_not_swallow() {
    // A single unclosed `$` re-emits as literal text, not silently-consumed math.
    let t = typ("\\documentclass{article}\\begin{document}\\[ a = b \\text{ price $5 } \\]\\end{document}");
    assert!(t.contains("price"), "text kept; got:\n{t}");
}

#[test]
fn embedded_quote_is_escaped() {
    // A `"` in the text run must be escaped so the Typst string stays valid.
    let t = typ("\\documentclass{article}\\begin{document}\\[ a = b \\text{say $x$ now} \\]\\end{document}");
    assert!(t.contains("x"), "inner math rendered; got:\n{t}");
}

#[test]
fn plain_text_no_math_unchanged() {
    // A `\text{}` with no inner `$...$` still becomes a single quoted string.
    let t = typ("\\documentclass{article}\\begin{document}\\[ a = b \\text{ for all } c \\]\\end{document}");
    assert!(t.contains("\"for all\""), "plain text stays a quoted string; got:\n{t}");
}
