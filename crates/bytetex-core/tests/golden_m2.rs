//! M2 golden tests: sectioning, inline formatting, lists, and misc commands.

use std::path::PathBuf;

use bytetex_core::{convert, ConvertOptions};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/bytetex-core has at least two parents")
        .join("tests/fixtures")
}

fn run(rel: &str) -> String {
    let path = fixtures_root().join(rel);
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    let opts = ConvertOptions {
        source_name: Some(rel.to_string()),
        ..Default::default()
    };
    let out = convert(&source, &opts);
    let warnings_json = serde_json::to_string_pretty(&out.warnings).expect("warnings serialize");
    format!(
        "==== TYPST ====\n{}==== WARNINGS ====\n{}\n",
        out.typst, warnings_json
    )
}

// ============== M2.1: sectioning ==============

#[test]
fn m2_sections_all_levels() {
    insta::assert_snapshot!(run("m2_sectioning/all_levels.tex"), @r"
    ==== TYPST ====
    = First Section

    Section body.

    == A Subsection

    Sub body.

    === A Subsubsection

    Subsub body.

    ==== Paragraph Heading

    Paragraph body.

    ===== Subparagraph Heading

    Subparagraph body.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m2_sections_starred() {
    insta::assert_snapshot!(run("m2_sectioning/starred.tex"), @r"
    ==== TYPST ====
    #heading(numbering: none)[Unnumbered Section]

    Body of an unnumbered section.

    #heading(level: 2, numbering: none)[Unnumbered Subsection]

    Sub body.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m2_sections_with_labels() {
    insta::assert_snapshot!(run("m2_sectioning/with_labels.tex"), @r"
    ==== TYPST ====
    = Introduction <sec:intro>

    Intro body.

    == Background <sec:bg>

    Background body.
    ==== WARNINGS ====
    []
    ");
}

// ============== M2.4: misc + document transparency ==============

#[test]
fn m2_misc_linebreaks() {
    // `\\` becomes Typst's `\` line break; `\noindent` and `\indent` drop silently.
    insta::assert_snapshot!(run("m2_misc/linebreaks.tex"), @r"
    ==== TYPST ====
    First line. \
    Second line after a forced break.

    A non-indented paragraph.

    An indented paragraph.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m2_misc_full_article() {
    // v0.2: `\documentclass` is silently dropped (always); `\usepackage` is
    // silently dropped when the package is in the known-noop allowlist
    // (inputenc is). The resulting Typst is clean — no warnings.
    insta::assert_snapshot!(run("m2_misc/full_article.tex"), @r#"
    ==== TYPST ====
    = Introduction <sec:intro>

    This article demonstrates *several* features at once: _italics_,
    #raw("monospace"), and section labels.

    - One.
    - Two.
    - Three.

    #heading(numbering: none)[Acknowledgments]

    We thank the #smallcaps[Authors] for everything.
    ==== WARNINGS ====
    []
    "#);
}

// ============== M2.3: lists ==============

#[test]
fn m2_list_itemize() {
    insta::assert_snapshot!(run("m2_lists/itemize.tex"), @r"
    ==== TYPST ====
    Before the list.

    - First item.
    - Second item with _italic_ text.
    - Third item.

    After the list.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m2_list_enumerate() {
    insta::assert_snapshot!(run("m2_lists/enumerate.tex"), @r"
    ==== TYPST ====
    + Numbered first.
    + Numbered second.
    + Numbered third.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m2_list_description() {
    insta::assert_snapshot!(run("m2_lists/description.tex"), @r"
    ==== TYPST ====
    / Alpha: First definition.
    / Beta: Second definition with *bold*.
    / Gamma: Third definition.
    ==== WARNINGS ====
    []
    ");
}

// ============== M2.2: inline formatting ==============

#[test]
fn m2_inline_basic() {
    insta::assert_snapshot!(run("m2_inline/basic.tex"), @r#"
    ==== TYPST ====
    A paragraph with _italics_, *bold*, _also italics_, and #raw("monospace") words.

    Another with #underline[underlined] and #smallcaps[Small Caps].
    ==== WARNINGS ====
    []
    "#);
}

#[test]
fn m2_inline_nested() {
    insta::assert_snapshot!(run("m2_inline/nested.tex"), @r"
    ==== TYPST ====
    Bold containing *outer _inner italic_ text* all together.

    Multiple wrappers: *_bold italic_* sample.
    ==== WARNINGS ====
    []
    ");
}

#[test]
fn m2_inline_in_heading() {
    insta::assert_snapshot!(run("m2_inline/in_heading.tex"), @r#"
    ==== TYPST ====
    = The _Curious_ Case of *Bold*

    Body of the section.

    #heading(level: 2, numbering: none)[Heading with #raw("code")]

    More body.
    ==== WARNINGS ====
    []
    "#);
}

#[test]
fn m2_sections_mixed_body() {
    insta::assert_snapshot!(run("m2_sectioning/mixed_body.tex"), @r"
    ==== TYPST ====
    = Methods

    First paragraph of methods.

    Second paragraph of methods,
    with a soft-wrapped line.

    == Setup

    Setup description.

    = Results

    We observed several things.
    ==== WARNINGS ====
    []
    ");
}
