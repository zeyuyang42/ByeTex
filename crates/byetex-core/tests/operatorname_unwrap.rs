//! `\operatorname{\mathrm{argmin}}` previously emitted `op("\mathrm{argmin}")`,
//! which renders the literal text `\mathrm{argmin}` (backslash and all) because
//! the inner is quoted verbatim as a Typst string. `op(...)` already renders its
//! argument upright, so a redundant `\mathrm{…}` / `\text{…}` / `\mbox{…}`
//! wrapper should be unwrapped to its content → `op("argmin")`. Common via
//! `\DeclareMathOperator*{\argmin}{\mathrm{argmin}}` (dogfood backlog, K1 follow-up).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn operatorname_unwraps_redundant_mathrm() {
    let t = typ(r"\documentclass{article}\begin{document}$\operatorname{\mathrm{argmin}}_x$\end{document}");
    assert!(t.contains(r#"op("argmin")"#), "should unwrap \\mathrm; got:\n{t}");
    assert!(!t.contains(r"\mathrm"), "redundant \\mathrm must be gone; got:\n{t}");
}

#[test]
fn operatorname_star_unwraps_redundant_mathrm() {
    let t = typ(r"\documentclass{article}\begin{document}$\operatorname*{\mathrm{argmax}}_y$\end{document}");
    assert!(
        t.contains(r#"op("argmax", limits: #true)"#),
        "starred should unwrap \\mathrm and keep limits; got:\n{t}"
    );
}

#[test]
fn operatorname_unwraps_text() {
    let t = typ(r"\documentclass{article}\begin{document}$\operatorname{\text{soft}}$\end{document}");
    assert!(t.contains(r#"op("soft")"#), "should unwrap \\text; got:\n{t}");
}

#[test]
fn operatorname_plain_name_unchanged() {
    // No wrapper — quoted as-is.
    let t = typ(r"\documentclass{article}\begin{document}$\operatorname{softmax}$\end{document}");
    assert!(t.contains(r#"op("softmax")"#), "plain name unchanged; got:\n{t}");
}
