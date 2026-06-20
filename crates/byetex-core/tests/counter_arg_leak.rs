//! `\addtocounter{c}{-1}` (and the counter family) leaked their `{c}{n}` args as body
//! text when a value like `-1` broke the `counter_*` node parse, so they fell to the
//! generic drop arm that assumed the args were children (round-4 dogfood A1, recurring).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn addtocounter_negative_does_not_leak() {
    let t = typ("\\documentclass{article}\\begin{document}\\addtocounter{footnote}{-1}\nBody text here.\n\\end{document}");
    assert!(t.contains("Body text here."), "body kept; got:\n{t}");
    assert!(!t.contains("addtocounter"), "command name must not leak; got:\n{t}");
    assert!(!t.contains("footnote") && !t.contains("-1"), "args must not leak; got:\n{t}");
}

#[test]
fn stepcounter_does_not_leak() {
    let t = typ("\\documentclass{article}\\begin{document}\\addtocounter{proposition}{-1}\nNext.\n\\end{document}");
    assert!(t.contains("Next."), "body kept");
    assert!(!t.contains("proposition"), "counter arg must not leak; got:\n{t}");
}

#[test]
fn following_paragraph_brace_group_is_kept() {
    // Regression: a brace group in a FOLLOWING paragraph is body, not a counter arg.
    let t = typ("\\documentclass{article}\\begin{document}\\addtocounter{x}{-1}\n\n{Important.}\n\\end{document}");
    assert!(t.contains("Important."), "following-paragraph group kept; got:\n{t}");
}
