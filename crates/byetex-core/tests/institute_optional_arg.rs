//! `\institute[short]{content}` — the optional `[short]` (common on beamer
//! title slides) makes tree-sitter parse `\institute` as a bare command with the
//! `[short]` and `{content}` as following siblings, so the handler's
//! `first_curly_like(node)` child lookup missed the content and it leaked into
//! the body (e.g. `\[\] Institute for Science…` on slide 2). The optional form
//! must be captured like the plain `\institute{content}` form (visual grader,
//! gh-klb2-beamer).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn institute_with_optional_arg_does_not_leak_into_body() {
    let src = r"\documentclass{beamer}\author{A}\institute[Short]{My University, Country}\begin{document}\begin{frame}Body text.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("Body text."), "lost body; got:\n{t}");
    // The institute content is captured (title block), not leaked into the body.
    assert!(t.contains("My University, Country"), "lost institute content; got:\n{t}");
    assert!(
        !t.contains(r"\[Short\]") && !t.contains(r"\[\]"),
        "leaked the optional-arg brackets; got:\n{t}"
    );
    // It must appear in the title-block `institution:` slot, not as a stray body line.
    assert!(t.contains("institution:"), "institute not captured into title block; got:\n{t}");
}

#[test]
fn institute_without_optional_arg_unchanged() {
    let t = typ(r"\documentclass{beamer}\author{A}\institute{My University}\begin{document}\begin{frame}Hi.\end{frame}\end{document}");
    assert!(t.contains("institution:") && t.contains("My University"), "plain institute broke; got:\n{t}");
}
