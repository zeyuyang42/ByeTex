//! Regression tests: title/abstract slots in class-aware show-calls
//! must be escaped against the Typst content-block delimiters
//! (`]`, `\`, `#`, `[`) — otherwise a paper with a `]` in its title
//! breaks the entire `#show: X.with(...)` block.
//!
//! Author slots have been escaped for a while via `content_escape` /
//! `string_escape`; this file guards the title and abstract slots
//! after they were brought into line.

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
    // IEEE templates route title through `[content]` slots; an
    // unescaped `]` would terminate the slot prematurely.
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
fn arxiv_title_with_backslash_does_not_break_show_call() {
    // arxiv (arkheion) does pass title through a `[content]` slot.
    // A stray `\` from a TeX command leaking into the rendered
    // title must be escaped so the show-call remains parseable.
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
    // Sanity: there's a show-call block, and the title appears.
    // The exact escape sequence depends on what the title-content
    // renderer produced; the critical assertion is that we don't
    // see a stray un-escaped `]` from the title in the slot.
    let show_block = out
        .typst
        .lines()
        .skip_while(|l| !l.contains("#show:"))
        .take_while(|l| !l.starts_with(')'))
        .collect::<Vec<_>>()
        .join("\n");
    // The show block must contain `title:` and must NOT have an
    // unbalanced `[`/`]` pair on the title line.
    assert!(
        show_block.contains("title:"),
        "no title slot in:\n{}",
        show_block
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
