//! Regression for Bug A (paper 2605.22814): a `\Cref`/`\ref` whose key contains
//! an underscore, used INSIDE a (sub)section title, makes tree-sitter truncate
//! the heading node early and orphan the FOLLOWING `\label{...}` as a sibling of
//! the parent section. The section's sibling-`\label` scanner then walked forward
//! across all intervening body content, grabbed that distant label, attached it
//! to the wrong heading, and advanced `skip_until` past it — silently deleting
//! every body node in between.
//!
//! This reproduces via a bare FRAGMENT (no \documentclass / document env),
//! because that is exactly how `\input`'d files are re-parsed and emitted in a
//! sub-emitter (see expand_latex_include): the file's `\section`/`\subsection`
//! sit at `source_file` top level, which is the parse shape that triggers the
//! early node-close.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn cref_with_underscore_in_subsection_title_does_not_eat_body() {
    // Bare fragment — mirrors an \input'd section file.
    let src = "\\section{Experiments}\n\
               \\label{sec:experiments}\n\
               EARLY BODY MARKER alpha.\n\
               \\subsection{Memory Ablations --- \\Cref{fig:memory_ablation}}\n\
               \\label{sec:ablation}\n\
               LATE BODY MARKER omega.\n";
    let out = convert(src);
    assert!(
        out.typst.contains("EARLY BODY MARKER alpha"),
        "section body before a Cref-in-title subsection was deleted; got:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("LATE BODY MARKER omega"),
        "body after the broken subsection was deleted; got:\n{}",
        out.typst
    );
    // The orphaned subsection label must not be hijacked onto the section heading.
    let exp_line = out
        .typst
        .lines()
        .find(|l| l.contains("Experiments"))
        .unwrap_or("");
    assert!(
        !exp_line.contains("<sec:ablation>"),
        "section heading wrongly absorbed the subsection's label; heading line: {:?}\nfull:\n{}",
        exp_line,
        out.typst
    );
}

#[test]
fn body_less_section_then_subsection_each_keep_own_label() {
    // A body-less `\section` immediately followed by a `\subsection`, each with
    // its own `\label`. The section's forward sibling-`\label` scanner must NOT
    // over-reach and steal the subsection's label (memory: the secondary
    // over-attachment concern from Bug A). Each heading keeps exactly its own.
    let out = convert(
        "\\section{Intro}\\label{sec:a}\n\\subsection{Sub}\\label{sec:b}\nbody\n",
    );
    assert!(
        out.typst.contains("= Intro <sec:a>"),
        "section must keep only <sec:a>, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("== Sub <sec:b>"),
        "subsection must keep its own <sec:b>, got: {}",
        out.typst
    );
}

#[test]
fn two_body_less_subsections_keep_own_labels() {
    let out = convert("\\subsection{A}\\label{x}\n\\subsection{B}\\label{y}\nbody\n");
    assert!(
        out.typst.contains("== A <x>") && out.typst.contains("== B <y>"),
        "each subsection must keep its own label, got: {}",
        out.typst
    );
}
