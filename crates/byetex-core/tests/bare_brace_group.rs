//! A bare LaTeX `{...}` group is scoping, not literal braces. ByeTex emitted
//! the braces verbatim, which Typst reads as a code block — e.g. after
//! `\appendix` (`#set heading(numbering: "A.1")`) the following
//! `{\Large\bfseries Appendix of \ours}` produced
//! `#set heading(numbering: "A.1"){ Appendix ... }` → "expected semicolon or
//! line break", and nested `\textbf{{X}}` leaked literal braces as `*{X}*`
//! (corpus 2605.31603). Fix: emit the inner content without the grouping braces.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn bare_group_drops_literal_braces() {
    let t = typst(r"{\Large\bfseries Appendix title}");
    assert!(t.contains("Appendix title"), "content must survive;\noutput:\n{t}");
    assert!(
        !t.contains('{') && !t.contains('}'),
        "a bare grouping `{{...}}` must not emit literal Typst braces;\noutput:\n{t}"
    );
}

#[test]
fn nested_braces_in_bold_do_not_leak() {
    let t = typst(r"\textbf{{Lumos-Nexus}}");
    assert!(
        t.contains("*Lumos-Nexus*"),
        "nested grouping braces inside bold must not leak;\noutput:\n{t}"
    );
    assert!(!t.contains("*{"), "no literal brace inside the bold;\noutput:\n{t}");
}

#[test]
fn appendix_heading_group_does_not_glue_to_set_rule() {
    let t = typst("\\appendix\n{\\Large\\bfseries Appendix of X}\nBody.");
    assert!(
        !t.contains("\"A.1\"){"),
        "the #set heading rule must not be glued to a `{{` code block;\noutput:\n{t}"
    );
}
