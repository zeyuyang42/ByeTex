//! Author-block parsing: end-to-end regression tests.
//!
//! Written TDD-style: all tests are designed to fail before the fix and pass
//! after. They exercise the full `convert()` path — the assertions reflect the
//! *target* behavior after the architectural fix is in place.
//!
//! Root cause summary: `\author{...}` content is currently fed through
//! `render_curly_group_content`, which causes the top-level command dispatcher
//! to intercept author sub-commands (`\email`, `\orcid`, `\corref`, `\fnref`,
//! `\And`, `\IEEEauthorblockN`, `\thanks`, …) before the per-author parser
//! ever sees them. The fix switches both `\author` entry points to raw-bytes
//! capture so the parsers in `class_map.rs` receive the actual LaTeX text.

use byetex_core::{convert, Category, ConvertOptions};

fn opts() -> ConvertOptions {
    ConvertOptions::default()
}

/// Wrap a preamble in a minimal article document. `\documentclass{article}`
/// routes to the `arkheion` template, so author fields appear in the
/// `#show: arkheion.with(authors: (…),)` call rather than in a bare
/// `#align(center)[…]` block.
fn article(preamble: &str) -> String {
    format!(
        r"\documentclass{{article}}{preamble}\title{{T}}\begin{{document}}Body.\end{{document}}"
    )
}

// ─── Test 1 ───────────────────────────────────────────────────────────────────

#[test]
fn author_corref_and_fnref_stripped() {
    // `\corref{tag}` and `\fnref{tag}` are Elsevier/elsearticle cross-ref
    // footnote markers. They have no display content and should be silently
    // stripped by the per-author parser.
    //
    // Before fix: no dispatch arm → `warn_unsupported_command` fires for each,
    //             but the body is consumed so the name string is "Alice" (clean).
    //             The problem is the spurious UnsupportedCommand warnings.
    // After fix:  raw bytes captured → parser strips both commands silently →
    //             no UnsupportedCommand warnings for \corref / \fnref.
    let src = article(r"\author{Alice\corref{c1}\fnref{f1}}");
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains("Alice"),
        "author name should survive corref/fnref strip; typst:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("corref"),
        "corref should not leak into typst output; typst:\n{}",
        out.typst
    );

    let has_corref_warn = out.warnings.iter().any(
        |w| matches!(&w.category, Category::UnsupportedCommand { name } if name == "\\corref"),
    );
    assert!(
        !has_corref_warn,
        "\\corref inside \\author must not emit UnsupportedCommand; warnings:\n{:#?}",
        out.warnings
    );

    let has_fnref_warn = out
        .warnings
        .iter()
        .any(|w| matches!(&w.category, Category::UnsupportedCommand { name } if name == "\\fnref"));
    assert!(
        !has_fnref_warn,
        "\\fnref inside \\author must not emit UnsupportedCommand; warnings:\n{:#?}",
        out.warnings
    );
}

// ─── Test 2 ───────────────────────────────────────────────────────────────────

#[test]
fn author_and_separator_splits_into_multiple_authors() {
    // `\And` (NeurIPS-style) is silently consumed by the dispatcher
    // (emit.rs:1700) before the author parser sees it, collapsing all
    // three names into one author entry.
    //
    // Before fix: one author "Alice  Bob  Carol" → no comma between names.
    // After fix:  raw bytes → parser splits on `\And` → three authors →
    //             comma-separated names in the arkheion.with(...) call.
    let src = article(r"\author{Alice \And Bob \And Carol}");
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "Alice should appear");
    assert!(out.typst.contains("Bob"), "Bob should appear");
    assert!(out.typst.contains("Carol"), "Carol should appear");

    // Three separate authors means Bob appears after a separator from Alice.
    // Before fix: "Alice  Bob  Carol" (one name, no separator in between).
    let alice_pos = out.typst.find("Alice").expect("Alice not found");
    let bob_pos = out.typst.find("Bob").expect("Bob not found");
    let between = &out.typst[alice_pos + "Alice".len()..bob_pos];
    assert!(
        between.contains(',') || between.contains('\n'),
        "Alice and Bob should be separated as distinct author entries; \
         between them: {:?}",
        between
    );
}

// ─── Test 3 ───────────────────────────────────────────────────────────────────

#[test]
fn author_ieee_block_name_extracted() {
    // `\IEEEauthorblockN{Name}` and `\IEEEauthorblockA{Affil}` have no
    // dispatch arm in the command table. The dispatcher falls through to
    // `warn_unsupported_command` and returns `node.end_byte()` without
    // emitting any content. The rendered author string is EMPTY, so Alice's
    // name is completely lost.
    //
    // Before fix: author name absent from typst output; two UnsupportedCommand
    //             warnings for \IEEEauthorblockN and \IEEEauthorblockA.
    // After fix:  raw bytes → parse_ieee_block sees the markers → name extracted.
    let src = r"\documentclass{IEEEtran}\author{\IEEEauthorblockN{Alice Smith}\IEEEauthorblockA{\textit{Dept of CS}\\MIT, USA\\alice@mit.edu}}\title{T}\begin{document}Body.\end{document}".to_string();
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains("Alice"),
        "IEEE author name should appear in typst output; typst:\n{}",
        out.typst
    );

    let has_block_warn = out.warnings.iter().any(|w| {
        matches!(&w.category, Category::UnsupportedCommand { name }
            if name == "\\IEEEauthorblockN" || name == "\\IEEEauthorblockA")
    });
    assert!(
        !has_block_warn,
        "IEEEauthorblockN/A inside \\author should not produce UnsupportedCommand; \
         warnings:\n{:#?}",
        out.warnings
    );
}

// ─── Test 4 ───────────────────────────────────────────────────────────────────

#[test]
fn author_thanks_no_footnote_leak_into_name() {
    // `\thanks{…}` currently fires the dispatcher arm at emit.rs:1820 which
    // rewrites it to `#footnote[…]` INSIDE the curly group render, so the
    // raw_authors string becomes "Alice#footnote[Equal contribution.]".
    // The per-author parser then cannot strip it and the footnote markup
    // leaks into the rendered author name.
    //
    // Before fix: typst contains `#footnote[Equal` embedded in the author block.
    // After fix:  raw bytes → parse_one_author strips `\thanks{…}` cleanly.
    let src = article(r"\author{Alice\thanks{Equal contribution.}}");
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "name should survive");
    assert!(
        !out.typst.contains("#footnote[Equal"),
        "\\thanks body should not leak as #footnote into the author name; \
         typst:\n{}",
        out.typst
    );
}

// ─── Test 5 ───────────────────────────────────────────────────────────────────

#[test]
fn author_email_not_stored_in_global_metadata() {
    // `\email{…}` inside `\author{}` currently hits the dispatcher at
    // emit.rs:1649 and is captured into the GLOBAL `metadata.class_metadata`
    // bag, losing per-author scope. The email is not rendered in the typst
    // output.
    //
    // Before fix: class_metadata["email"] = "alice@example.org";
    //             typst does NOT contain the address.
    // After fix:  raw bytes → parser assigns email to Author.email;
    //             class_metadata["email"] is absent;
    //             typst shows the address (arkheion template renders it).
    let src = article(r"\author{Alice\email{alice@example.org}}");
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "name should survive");

    assert!(
        !out.class_metadata.contains_key("email"),
        "email inside \\author should NOT land in global class_metadata; \
         got: {:?}",
        out.class_metadata.get("email")
    );

    // `post_process_typography` escapes `@` → `\@` in the assembled Typst
    // output, so the email may appear as "alice\@example.org" in a string slot.
    // Either form confirms the email reached the author record.
    assert!(
        out.typst.contains("alice@example.org") || out.typst.contains("alice\\@example.org"),
        "email should appear in typst output via the author record; \
         typst:\n{}",
        out.typst
    );
}

// ─── Test 6 ───────────────────────────────────────────────────────────────────

#[test]
fn author_orcid_in_typst_output() {
    // `\orcid{…}` inside `\author{}` currently goes to class_metadata["orcid"]
    // and is never rendered. After fix: assigned to Author.orcid and rendered
    // by the template (arkheion includes the orcid field).
    let src = article(r"\author{Alice\orcid{0000-0001-2345-6789}}");
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "name should survive");

    assert!(
        !out.class_metadata.contains_key("orcid"),
        "orcid inside \\author should NOT land in global class_metadata"
    );

    assert!(
        out.typst.contains("0000-0001-2345-6789"),
        "orcid should appear in typst output; typst:\n{}",
        out.typst
    );
}

// ─── Test 7 ───────────────────────────────────────────────────────────────────

#[test]
fn author_latex_accent_escape_rendered() {
    // After raw-bytes capture, LaTeX accent sequences like `\"u` arrive in
    // the per-author parser as raw text. The `latex_text_to_typst` helper
    // must convert them to the Unicode equivalent before storing.
    //
    // Before fix: render_curly_group_content handles the accent, so this
    //             may already pass. After step 1 (raw bytes) alone this BREAKS
    //             because `\"u` is no longer processed. Step 3 (`latex_text_to_typst`)
    //             restores correctness.
    let src = article(r#"\author{M\"uller \and Sch\"afer}"#);
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains('ü') || out.typst.contains("Müller"),
        "ü should be rendered (not kept as \\\"u); typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains('ä') || out.typst.contains("Schäfer"),
        "ä should be rendered (not kept as \\\"a); typst:\n{}",
        out.typst
    );
    // Raw escape must not appear verbatim.
    assert!(
        !out.typst.contains(r#"\""#),
        "raw LaTeX accent \\\" must not appear in typst output; typst:\n{}",
        out.typst
    );
}

// ─── Test 8 ───────────────────────────────────────────────────────────────────

#[test]
fn author_pdf_metadata_set_document() {
    // `finish()` should emit `#set document(author: …)` so the PDF metadata
    // field is populated. Currently there is no such line in the output.
    let src = article(r"\author{Alice Doe \and Bob Smith}");
    let out = convert(&src, &opts());

    assert!(
        out.typst.contains("#set document(author:"),
        "typst output should contain `#set document(author: …)` for PDF metadata; \
         typst:\n{}",
        out.typst
    );
    assert!(
        out.typst.contains("Alice"),
        "Alice should appear in the set document author list"
    );
    assert!(
        out.typst.contains("Bob"),
        "Bob should appear in the set document author list"
    );
}

// ─── Test 9 ───────────────────────────────────────────────────────────────────

#[test]
fn author_unknown_subcommand_no_unsupported_command_warning() {
    // After the raw-bytes fix, the top-level dispatcher never sees commands
    // inside `\author{}`. Unknown subcommands should NOT emit a generic
    // `UnsupportedCommand` warning (they reach the per-author parser which
    // silently strips them or emits `AuthorFieldDropped`).
    //
    // Before fix: \unknowncmd falls through the dispatcher → UnsupportedCommand.
    // After fix:  raw bytes → per-author parser handles/strips it → no UnsupportedCommand.
    let src = article(r"\author{Alice\unknowncmd{x}}");
    let out = convert(&src, &opts());

    assert!(out.typst.contains("Alice"), "author name should survive");

    let has_unsupported = out.warnings.iter().any(
        |w| matches!(&w.category, Category::UnsupportedCommand { name } if name == "\\unknowncmd"),
    );
    assert!(
        !has_unsupported,
        "\\unknowncmd inside \\author should NOT emit UnsupportedCommand; \
         got warnings:\n{:#?}",
        out.warnings
    );
}
