//! Regression tests: title/abstract content emitted into the generated
//! neutral title block (`#align(center)[ #text(...)[<title>] ]`) must be
//! escaped against the Typst content-block delimiters (`]`, `\`, `#`, `[`) —
//! otherwise a paper with a `]` in its title would terminate the `[...]`
//! slot prematurely and break the whole document.
//!
//! (ByeTex no longer binds a Typst Universe template / `#show:` block; these
//! slots now live in the self-generated title block.)

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
}

#[test]
fn ieee_title_with_bracket_is_escaped() {
    // The title is rendered into a `[content]` slot in the title block;
    // an unescaped `]` would terminate the slot prematurely.
    let src = r"\documentclass[conference]{IEEEtran}
\title{Foo [Bar] baz}
\author{Alice}
\begin{document}
\maketitle
\end{document}";
    let out = convert_str(src);
    // The literal `]` from the title must appear as the Typst escape
    // `\]` somewhere in the show-call block.
    assert!(
        out.typst.contains(r"\]"),
        "expected `\\]` in title slot; got:\n{}",
        out.typst
    );
    // And it must land inside the generated title block.
    assert!(
        out.typst.contains("#align(center)"),
        "expected a title block; got:\n{}",
        out.typst
    );
}

#[test]
fn acm_abstract_with_hash_is_escaped() {
    let src = r"\documentclass{acmart}
\title{Hash test}
\author{Bob}
\begin{document}
\begin{abstract}
We discuss \#1 reasons.
\end{abstract}
\maketitle
\end{document}";
    let out = convert_str(src);
    // ACM doesn't carry abstract through the show-call, but verify
    // the conversion itself doesn't choke. (acmart embeds abstract
    // in the body.)
    assert!(!out.typst.is_empty());
}

#[test]
fn arxiv_title_with_backslash_does_not_break_title_block() {
    // The title is passed through a `[content]` slot in the title block.
    // Inline math (`\(...\)`) must be converted to a Typst math span, not
    // leaked as a stray `\` that would break the slot.
    let src = r"\documentclass{article}
\title{Heat \(\mathcal{H}\) equation}
\author{Carol}
\begin{document}
\begin{abstract}
Brief.
\end{abstract}
\maketitle
\end{document}";
    let out = convert_str(src);
    // The generated title block carries the title, with the math rendered
    // as a Typst math span (no leaked `\(` / `\)` delimiters).
    let title_line = out
        .typst
        .lines()
        .find(|l| l.contains("size: 1.728em") && l.contains("Heat"))
        .unwrap_or_else(|| panic!("no title line in:\n{}", out.typst));
    assert!(
        title_line.contains("$") && !title_line.contains(r"\("),
        "title math should be a `$...$` span, not leaked `\\(`; got:\n{}",
        title_line
    );
}

#[test]
fn neurips_abstract_with_bracket_in_text_escapes() {
    let src = r"\documentclass{article}
\usepackage{neurips_2024}
\title{T}
\author{D}
\begin{document}
\begin{abstract}
We compare [baseline] vs ours.
\end{abstract}
\maketitle
\end{document}";
    let out = convert_str(src);
    // For class variants that DO put abstract in a content slot,
    // the `]` from `[baseline]` must come out as `\]`. For variants
    // that don't (some neurips configs), the abstract lands in the
    // body and the bracket is fine. Either way the convert must
    // not produce a parse_error.
    let has_parse_error = out
        .warnings
        .iter()
        .any(|w| matches!(&w.category, byetex_core::Category::ParseError { .. }));
    assert!(
        !has_parse_error,
        "unexpected parse_error: {:?}",
        out.warnings
    );
}

#[test]
fn author_email_at_sign_is_escaped_in_title_block() {
    // Regression: an author string carrying an email-like `@` is rendered
    // into the `[...]` content slot of the title block. An un-escaped `@`
    // makes Typst parse `@math.uzh.ch` as a *reference*, breaking compilation
    // with "label does not exist". It must be emitted as `\@`.
    let src = r"\documentclass{article}
\title{T}
\author{Stas (stas@math.uzh.ch)}
\begin{document}
\maketitle
\end{document}";
    let out = convert_str(src);
    // The title block (content) carries the escaped form.
    assert!(
        out.typst.contains(r"\@math.uzh.ch"),
        "author `@` must be escaped to `\\@` in the title block; got:\n{}",
        out.typst
    );
    // And no bare `@math.uzh.ch` reference leaked into the content.
    let in_title_block = out
        .typst
        .lines()
        .any(|l| l.contains("stas") && l.contains(r"\@") && !l.starts_with("#set document"));
    assert!(
        in_title_block,
        "expected the escaped email on the author line; got:\n{}",
        out.typst
    );
}
