//! A custom `\newenvironment` must not drop its body. tree-sitter parses the
//! declaration as an `environment_definition` node; previously it was unhandled
//! and leaked its raw source, while any `\begin{name}...\end{name}` use hit the
//! unknown-env arm and dropped the whole body — including any `\label` inside,
//! so `@key` references dangled ("label does not exist"). arXiv:2605.22765 hit
//! this (a `\label` inside `\begin{hypA}`).
//!
//! Fix: register the env name as a transparent (empty-display) kind so its body
//! passes through, and emit nothing for the definition itself.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// The body (text + label) of a custom `\newenvironment` survives, the label
/// becomes a referenceable anchor, and the definition does not leak.
#[test]
fn newenvironment_body_and_label_survive() {
    let src = "\\newenvironment{hypA}{\\refstepcounter{hypA}\\begin{itemize}\
        \\item[(A\\arabic{hypA})]}{\\end{itemize}}\n\
        \\begin{hypA}\\label{ass:x}\nBody text here.\n\\end{hypA}\n\
        See \\ref{ass:x}.";
    let t = typst(src);

    // Body content preserved.
    assert!(
        t.contains("Body text here."),
        "the custom env body must pass through;\noutput:\n{t}"
    );
    // Label preserved and referenceable (anchor from PR #132).
    assert!(
        t.contains("<ass:x>") && t.contains("kind: \"anchor\""),
        "the label inside the env must survive as a referenceable anchor;\noutput:\n{t}"
    );
    // The reference resolves to the same key.
    assert!(
        t.contains("@ass:x"),
        "the reference must be emitted;\noutput:\n{t}"
    );
    // The definition itself must not leak into the body.
    assert!(
        !t.contains("\\newenvironment") && !t.contains("refstepcounter"),
        "the \\newenvironment definition must not leak;\noutput:\n{t}"
    );
}

/// `\renewenvironment` is handled the same way.
#[test]
fn renewenvironment_body_survives() {
    let src = "\\renewenvironment{mybox}{\\par}{\\par}\n\
        \\begin{mybox}\nImportant content.\n\\end{mybox}";
    let t = typst(src);
    assert!(
        t.contains("Important content."),
        "renewenvironment body must pass through;\noutput:\n{t}"
    );
    assert!(
        !t.contains("\\renewenvironment"),
        "the \\renewenvironment definition must not leak;\noutput:\n{t}"
    );
}

/// A custom env with no `\label` and plain text still passes through with no
/// UnsupportedEnvironment warning.
#[test]
fn custom_env_use_does_not_warn() {
    let src = "\\newenvironment{notebox}{}{}\n\\begin{notebox}\nHello.\n\\end{notebox}";
    let r = convert(src, &ConvertOptions::default());
    assert!(r.typst.contains("Hello."), "body preserved;\n{}", r.typst);
    assert!(
        !r.warnings
            .iter()
            .any(|w| format!("{:?}", w.category).contains("notebox")),
        "a defined custom env must not warn as unsupported;\nwarnings: {:?}",
        r.warnings.iter().map(|w| &w.category).collect::<Vec<_>>()
    );
}
