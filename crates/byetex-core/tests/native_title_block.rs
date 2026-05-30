//! Rich native title-block renderer: TDD regression tests.
//!
//! These tests exercise `flush_title_block` on the fallback path
//! (Unknown / Lncs / SvMult — classes with no Typst Universe template).
//! All tests are written to fail before the implementation and pass after.
//!
//! Fallback trigger: `\documentclass{amsart}` maps to `DocClass::Unknown`
//! so `import_line()` returns None and `flush_title_block` runs.

use byetex_core::{convert, ConvertOptions};

fn opts() -> ConvertOptions {
    ConvertOptions::default()
}

/// Wrap preamble + body in an amsart document (Unknown class → fallback path).
fn fallback_doc(preamble: &str, body: &str) -> String {
    format!(r"\documentclass{{amsart}}{preamble}\begin{{document}}{body}\end{{document}}")
}

// ─── Test 1 ───────────────────────────────────────────────────────────────────

#[test]
fn native_affiliation_rendered() {
    // Currently flush_title_block only emits author names — affiliations are
    // silently dropped. After the fix the affiliation text should appear.
    let src = fallback_doc(
        r"\title{T}\author{Alice\affiliation{MIT, Cambridge}}",
        "Body.",
    );
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "author name should appear");
    assert!(
        out.typst.contains("MIT"),
        "affiliation should appear in typst output; typst:\n{}",
        out.typst
    );
}

// ─── Test 2 ───────────────────────────────────────────────────────────────────

#[test]
fn native_abstract_rendered_as_styled_block() {
    // Currently wants_abstract_field() returns false for Unknown, so the
    // abstract environment is emitted as plain inline text — no styled block,
    // no "Abstract" header. After the fix the abstract should be captured and
    // rendered with a visible "Abstract" label.
    let src = fallback_doc(
        r"\title{T}\author{Alice}",
        r"\begin{abstract}This is the abstract content.\end{abstract}Body.",
    );
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains("This is the abstract content"),
        "abstract text should appear; typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("Abstract"),
        "typst output should include an 'Abstract' header/label; typst:\n{}",
        out.typst
    );
}

// ─── Test 3 ───────────────────────────────────────────────────────────────────

#[test]
fn native_keywords_rendered() {
    // Currently \keywords for Unknown class hits warn_silently_dropped and
    // is absent from the output. After the fix keywords should appear.
    let src = fallback_doc(
        r"\title{T}\author{Alice}\keywords{machine learning, deep learning}",
        "Body.",
    );
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains("machine learning"),
        "keywords should appear in typst output; typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("deep learning"),
        "all keywords should appear; typst:\n{}",
        out.typst
    );
}

// ─── Test 4 ───────────────────────────────────────────────────────────────────

#[test]
fn native_orcid_rendered() {
    // \orcid{...} inside \author{} is captured into Author.orcid by
    // parse_one_author, but flush_title_block currently never emits it.
    // After the fix the ORCID should appear in the output (as a link or ID).
    let src = fallback_doc(
        r"\title{T}\author{Alice\orcid{0000-0001-2345-6789}}",
        "Body.",
    );
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "author name should appear");
    assert!(
        out.typst.contains("0000-0001-2345-6789"),
        "ORCID should appear in typst output; typst:\n{}",
        out.typst
    );
}

// ─── Test 5 ───────────────────────────────────────────────────────────────────

#[test]
fn native_email_rendered() {
    // \email{...} inside \author{} is captured into Author.email, but
    // flush_title_block currently never emits it.
    // post_process_typography escapes @ → \@, so accept either form.
    let src = fallback_doc(r"\title{T}\author{Alice\email{alice@example.com}}", "Body.");
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "author name should appear");
    assert!(
        out.typst.contains("alice@example.com") || out.typst.contains("alice\\@example.com"),
        "email should appear in typst output (raw or @-escaped); typst:\n{}",
        out.typst
    );
}

// ─── Test 6 ───────────────────────────────────────────────────────────────────

#[test]
fn native_shared_affiliation_deduplicated() {
    // Two authors sharing the same affiliation should produce one affiliation
    // entry in the footer, not two. The current renderer never emits
    // affiliations at all, so after the fix we need exactly one "MIT" entry.
    let src = fallback_doc(
        r"\title{T}\author{Alice\affiliation{MIT} \and Bob\affiliation{MIT}}",
        "Body.",
    );
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "Alice should appear");
    assert!(out.typst.contains("Bob"), "Bob should appear");

    let mit_count = out.typst.matches("MIT").count();
    assert_eq!(
        mit_count, 1,
        "shared affiliation 'MIT' should appear exactly once; typst:\n{}",
        out.typst
    );
}

// ─── Test 7 ───────────────────────────────────────────────────────────────────

#[test]
fn native_no_affiliation_no_superscripts() {
    // When no author has an affiliation, the title block should render just
    // the names — no `#super[...]` calls, no blank affiliation footer.
    let src = fallback_doc(r"\title{T}\author{Alice \and Bob \and Carol}", "Body.");
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "Alice should appear");
    assert!(out.typst.contains("Bob"), "Bob should appear");
    assert!(out.typst.contains("Carol"), "Carol should appear");

    assert!(
        !out.typst.contains("#super["),
        "no affiliations → no superscripts should appear; typst:\n{}",
        out.typst
    );
}

// ─── Test 8 ───────────────────────────────────────────────────────────────────

#[test]
fn native_date_rendered() {
    // Date should still appear in the fallback title block after the
    // implementation is expanded. (This tests that the refactor doesn't regress
    // existing date behaviour.)
    let src = fallback_doc(r"\title{T}\author{Alice}\date{January 2025}", "Body.");
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains("January 2025"),
        "date should appear in typst output; typst:\n{}",
        out.typst
    );
}
