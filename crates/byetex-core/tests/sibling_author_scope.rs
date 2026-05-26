//! D6: per-author sibling-scope attribution.
//!
//! LaTeX styles like elsearticle and authblk place \email / \orcid /
//! \affil / \address as *siblings* of \author{} rather than nested inside
//! it.  The current code writes all of them to class_metadata with
//! first-write-wins semantics, so only the first author's email survives.
//!
//! After the fix, each sibling command is appended to the most-recently-
//! seen \author{} raw buffer and is parsed by the per-author parser — so
//! every author gets their own structured fields.
//!
//! These tests use \documentclass{amsart} (Unknown class → flush_title_block)
//! to keep the assertions independent of template field shapes.

use byetex_core::{convert, ConvertOptions};

fn opts() -> ConvertOptions {
    ConvertOptions::default()
}

fn doc(preamble: &str) -> String {
    format!(r"\documentclass{{amsart}}{preamble}\title{{T}}\begin{{document}}Body.\end{{document}}")
}

// ─── Test 1 ───────────────────────────────────────────────────────────────────

#[test]
fn sibling_email_both_authors_get_email() {
    // Currently \email{bob@y.com} is blocked by or_insert (first-write-wins)
    // so Bob's email is silently lost.
    // After fix: each email is appended to the preceding author's raw buffer.
    let src = doc(r"\author{Alice}\email{alice@x.com}\author{Bob}\email{bob@y.com}");
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains("alice@x.com") || out.typst.contains("alice\\@x.com"),
        "Alice's email should appear; typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("bob@y.com") || out.typst.contains("bob\\@y.com"),
        "Bob's email should appear (currently lost); typst:\n{}",
        out.typst
    );
}

// ─── Test 2 ───────────────────────────────────────────────────────────────────

#[test]
fn sibling_affil_each_author_gets_own_affiliation() {
    // authblk pattern: \author{Name}\affil{Institution}
    // Currently both \affil{...} calls go to class_metadata["affil"] and only
    // the first survives; the second is silently dropped.
    let src = doc(r"\author{Alice}\affil{MIT}\author{Bob}\affil{Stanford}");
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "Alice should appear");
    assert!(out.typst.contains("Bob"), "Bob should appear");
    assert!(
        out.typst.contains("MIT"),
        "Alice's affiliation MIT should appear; typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("Stanford"),
        "Bob's affiliation Stanford should appear (currently lost); typst:\n{}",
        out.typst
    );
}

// ─── Test 3 ───────────────────────────────────────────────────────────────────

#[test]
fn sibling_orcid_both_authors_get_orcid() {
    // Each author's ORCID should appear in the output, not just the first.
    let src = doc(
        r"\author{Alice}\orcid{0000-0001-0000-0000}\author{Bob}\orcid{0000-0002-0000-0000}",
    );
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains("0000-0001-0000-0000"),
        "Alice's ORCID should appear; typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("0000-0002-0000-0000"),
        "Bob's ORCID should appear (currently lost); typst:\n{}",
        out.typst
    );
}

// ─── Test 4 ───────────────────────────────────────────────────────────────────

#[test]
fn sibling_affil_not_in_class_metadata() {
    // After the fix the per-author fields should NOT land in class_metadata —
    // they belong to the structured Author record.
    let src = doc(r"\author{Alice}\affil{MIT}");
    let out = convert(&src, &opts());

    assert!(
        out.class_metadata.get("affil").is_none(),
        "\\affil inside sibling scope should NOT land in class_metadata; \
         got: {:?}",
        out.class_metadata.get("affil")
    );
    assert!(
        out.typst.contains("MIT"),
        "affiliation should still appear in typst output; typst:\n{}",
        out.typst
    );
}

// ─── Test 5 ───────────────────────────────────────────────────────────────────

#[test]
fn sibling_email_no_author_context_does_not_crash() {
    // A \email{...} with no preceding \author{} should not panic or crash —
    // it should be silently dropped (no author to attach it to).
    let src = doc(r"\email{orphan@x.com}\author{Alice}");
    let out = convert(&src, &opts());

    // The orphaned email may or may not appear — we just require no panic
    // and that Alice still appears.
    assert!(out.typst.contains("Alice"), "Alice should appear");
}
