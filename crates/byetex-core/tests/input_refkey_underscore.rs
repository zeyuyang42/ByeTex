//! `ir::neutralize_ref_key_underscores` (PR #452) protects the ENTRY file from
//! the tree-sitter parse cascade that underscores inside `\label`/`\ref` keys
//! trigger. It was only applied in `convert_with_macros`, so every `\input`'ed
//! file was parsed raw — meaning the layout project mode exists for
//! (chapter-per-file) still lost most of its document.
//!
//! Review finding #1: the same body must convert identically whether it sits in
//! the entry file or in an `\input`'ed one.
//!
//! The body below is the delta-debugged minimisation of corpus 2605.22728,
//! where the whole-file conversion recovered 8 headings but the identical body
//! behind an `\input` recovered only 2. Unbalanced braces inside a `\smash{…}`
//! math run push tree-sitter into an ERROR node; the underscore in the
//! following `\label` key is what makes that error swallow the rest of the
//! file, so neutralising it pre-parse is what keeps the headings.

use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

const BODY: &str = concat!(
    "\\,\\smash{\\bigl\\{\\tfrac{1}{x}}y\\bigr\\}\\\\\n",
    "\\section{Alpha}\\label{sec:continuous}\n",
    "\\subsection{Beta}\\label{subsec:rof_continuous}\n",
);

fn count_headings(typst: &str) -> usize {
    typst
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with('=') && t.trim_start_matches('=').starts_with(' ')
        })
        .count()
}

fn convert_in(dir: &std::path::Path, name: &str, source: &str) -> byetex_core::ConvertOutput {
    convert(
        source,
        &ConvertOptions {
            source_name: Some(name.to_string()),
            base_dir: Some(PathBuf::from(dir)),
        },
    )
}

#[test]
fn included_file_gets_the_same_refkey_neutralization_as_the_entry_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    // (a) everything in the entry file — the path that already neutralises.
    let flat = format!("\\documentclass{{article}}\n\\begin{{document}}\n{BODY}\\end{{document}}\n");
    let flat_out = convert_in(dir, "flat.tex", &flat);
    let flat_headings = count_headings(&flat_out.typst);
    assert_eq!(
        flat_headings, 2,
        "entry-file baseline changed:\n{}",
        flat_out.typst
    );

    // (b) the identical body behind an \input.
    std::fs::write(dir.join("body.tex"), BODY).unwrap();
    let split_out = convert_in(
        dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\n\\input{body}\n\\end{document}\n",
    );
    assert_eq!(
        count_headings(&split_out.typst),
        flat_headings,
        "\\input'ed body lost headings the flat conversion kept:\n{}",
        split_out.typst
    );
}

#[test]
fn included_file_output_carries_no_sentinel_character() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(dir.join("body.tex"), BODY).unwrap();
    let out = convert_in(
        dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\n\\input{body}\n\\end{document}\n",
    );
    assert!(
        !out.typst.contains('\u{1f}'),
        "refkey sentinel leaked into output:\n{:?}",
        out.typst
    );
}

#[test]
fn included_file_label_keys_keep_their_underscores() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    std::fs::write(
        dir.join("body.tex"),
        "\\section{Alpha}\\label{sec:my_section}\nSee \\ref{sec:my_section}.\n",
    )
    .unwrap();
    let out = convert_in(
        dir,
        "main.tex",
        "\\documentclass{article}\n\\begin{document}\n\\input{body}\n\\end{document}\n",
    );
    assert!(
        out.typst.contains("sec:my_section"),
        "label key mangled in included file:\n{}",
        out.typst
    );
}
