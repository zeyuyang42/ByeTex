//! The `\color{name}` *switch* form (vs `\textcolor{name}{content}`) leaked the
//! colour name as body text. tree-sitter parses `{\color{red}text}` as a
//! `color_reference` (`\color{red}`) followed by a *sibling* `text`, but
//! `emit_textcolor` took the last `{…}` group as the content — which for the
//! switch form is the colour itself — so it emitted the colour name (`red`,
//! or an unresolvable blend like `bgcolorAlt!90!fgcolor`) as text. The switch
//! form has no content; drop it (visual grader, gh-klb2-beamer `\seprule`).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn color_switch_does_not_leak_name() {
    let t = typ(r"\documentclass{article}\begin{document}{\color{red}hello} world\end{document}");
    assert!(t.contains("hello") && t.contains("world"), "lost content; got:\n{t}");
    // The colour name must not appear as standalone body text.
    assert!(!t.contains("redhello") && !t.lines().any(|l| l.contains("red") && l.contains("hello")),
        "leaked the colour name; got:\n{t}");
}

#[test]
fn color_switch_unresolvable_blend_does_not_leak() {
    let t = typ(r"\documentclass{article}\definecolor{bgcolorAlt}{HTML}{ECF1FC}\begin{document}{\color{bgcolorAlt!90!fgcolor}X}\end{document}");
    assert!(t.contains("X"), "lost content; got:\n{t}");
    assert!(!t.contains("bgcolorAlt"), "leaked the blend expression; got:\n{t}");
}

#[test]
fn textcolor_wrap_form_still_colors() {
    let t = typ(r"\documentclass{article}\begin{document}\textcolor{red}{hi}\end{document}");
    assert!(t.contains("#text(fill:") && t.contains("[hi]"), "textcolor wrap broke; got:\n{t}");
}
