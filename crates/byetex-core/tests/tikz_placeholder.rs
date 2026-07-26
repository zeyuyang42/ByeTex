//! A `tikzpicture` must never vanish silently.
//!
//! The documented contract — README's "never hard-fails" bullet and
//! `docs/conversion-logic.md` — is that anything ByeTex can't translate becomes
//! a VISIBLE placeholder plus a STRUCTURED warning. `tikzpicture` was the one
//! construct that broke it: the body was dropped with no placeholder and no
//! warning at all, so a paper whose figures are TikZ lost them with zero signal
//! (`warnings.json` came back `[]`).
//!
//! TikZ is ubiquitous in academic LaTeX, so this is the most likely first
//! thing a new user hits.

use byetex_core::{convert, Category, ConvertOptions};

const TIKZ: &str = r#"\documentclass{article}
\usepackage{tikz}
\begin{document}
Before.
\begin{tikzpicture}
  \draw (0,0) -- (2,2);
  \node at (1,1) {hello};
\end{tikzpicture}
After.
\end{document}
"#;

#[test]
fn a_tikzpicture_produces_a_warning() {
    let out = convert(TIKZ, &ConvertOptions::default());
    assert!(
        !out.warnings.is_empty(),
        "a dropped tikzpicture must be reported, got no warnings at all"
    );
    assert!(
        out.warnings
            .iter()
            .any(|w| matches!(w.category, Category::Tikz)),
        "expected a Category::Tikz warning, got {:?}",
        out.warnings.iter().map(|w| &w.category).collect::<Vec<_>>()
    );
}

#[test]
fn a_tikzpicture_warning_routes_to_the_tikz_skill() {
    // `byetex-tikz-to-typst` ships in the bundle but was unreachable via
    // `suggested_skill` because Category::Tikz was never constructed.
    let out = convert(TIKZ, &ConvertOptions::default());
    let tikz = out
        .warnings
        .iter()
        .find(|w| matches!(w.category, Category::Tikz))
        .expect("a Tikz warning");
    assert_eq!(
        tikz.suggested_skill.as_deref(),
        Some("byetex-tikz-to-typst"),
        "the Tikz warning must point at the TikZ repair skill"
    );
}

#[test]
fn a_tikzpicture_leaves_a_visible_placeholder() {
    let out = convert(TIKZ, &ConvertOptions::default());
    let body = &out.typst;
    assert!(
        body.contains("tikzpicture"),
        "the reader must see that a drawing was here:\n{body}"
    );
    // Surrounding prose must survive.
    assert!(body.contains("Before."), "text before the picture lost");
    assert!(body.contains("After."), "text after the picture lost");
}

#[test]
fn the_tikz_body_itself_does_not_leak_as_text() {
    // The point of dropping the body is that `\draw (0,0) -- (2,2);` is not
    // prose. A placeholder must not become a dump of the drawing commands.
    let out = convert(TIKZ, &ConvertOptions::default());
    assert!(
        !out.typst.contains("(0,0) -- (2,2)"),
        "raw TikZ drawing commands leaked into the body:\n{}",
        out.typst
    );
}

#[test]
fn the_output_still_compiles_as_typst_markup() {
    // The placeholder must be valid markup, not a bare backslash command.
    let out = convert(TIKZ, &ConvertOptions::default());
    for line in out.typst.lines() {
        assert!(
            !line.trim_start().starts_with("\\begin{"),
            "raw LaTeX leaked: {line}"
        );
    }
}
