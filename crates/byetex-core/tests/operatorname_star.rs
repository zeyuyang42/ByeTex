//! Regression: the starred `\operatorname*{X}` (limits-above form, e.g.
//! `\operatorname*{argmin}_x`) was unhandled — tree-sitter includes the `*` in
//! the command name (`\operatorname*`), so the `\operatorname` dispatch arm
//! missed it and the call fell through to generic handling, emitting the bare
//! string `operatorname*` with the `{X}` argument dropped (dogfood backlog K1,
//! 2605.22821). It must render `op("X", limits: #true)` like the plain form
//! renders `op("X")`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn operatorname_plain_unchanged() {
    let t = typ(r"\documentclass{article}\begin{document}$\operatorname{argmin}_x f$\end{document}");
    assert!(t.contains(r#"op("argmin")"#), "plain operatorname; got:\n{t}");
}

#[test]
fn operatorname_star_keeps_argument_with_limits() {
    let t = typ(r"\documentclass{article}\begin{document}$\operatorname*{argmin}_x f$\end{document}");
    assert!(
        t.contains(r#"op("argmin", limits: #true)"#),
        "starred operatorname should emit op with limits; got:\n{t}"
    );
    assert!(
        !t.contains("operatorname*") && !t.contains("\"operatorname"),
        "the command name must not leak as a string; got:\n{t}"
    );
}
