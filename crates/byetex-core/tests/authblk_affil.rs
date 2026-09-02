//! authblk's `\affil[n]{body}` (and `\affil{body}`) must not leak its optional
//! `[n]` index or its body into the document. tree-sitter-latex parses the
//! optional `[n]` + `{body}` as siblings of the bare `\affil` generic_command,
//! and the old handler captured the body via `first_curly_like` but never
//! advanced `skip_until`, so both leaked as raw text (dogfood 2605.22728 / and
//! cleanly on 2605.22724, 2605.31394: `\affil[1]{Dept...}` → `\[1\]Dept...`).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
    .typst
}

#[test]
fn affil_with_optional_index_does_not_leak() {
    let src = r"\documentclass{article}\usepackage{authblk}
\author[1]{Alice}
\affil[1]{Department of Mathematics, Example University}
\begin{document}
Body paragraph.
\end{document}";
    let t = typ(src);
    // The escaped optional-index artifact must NOT appear.
    assert!(
        !t.contains(r"\[1\]"),
        "optional `[1]` leaked as `\\[1\\]`:\n{t}"
    );
    // The raw command must NOT appear in the output.
    assert!(!t.contains(r"\affil"), "raw `\\affil` leaked:\n{t}");
    // The affiliation text must NOT appear inside the body paragraph region
    // (it belongs in the author block). The body sentinel is "Body paragraph."
    let body_idx = t.find("Body paragraph.").expect("body present");
    assert!(
        !t[body_idx..].contains("Department of Mathematics"),
        "affiliation body leaked into the document body:\n{t}"
    );
}

#[test]
fn affil_without_optional_arg_does_not_leak() {
    let src = r"\documentclass{article}\usepackage{authblk}
\author{Bob}
\affil{CERN, Geneva}
\begin{document}
Body paragraph.
\end{document}";
    let t = typ(src);
    assert!(!t.contains(r"\affil"), "raw `\\affil` leaked:\n{t}");
    let body_idx = t.find("Body paragraph.").expect("body present");
    assert!(
        !t[body_idx..].contains("CERN, Geneva"),
        "affiliation body leaked into the document body:\n{t}"
    );
}

// ── The `[n]` index must be HONOURED, not merely not-leaked ─────────────────
//
// The handler skipped `[n]` and appended every body to the LAST author seen, so
// with two authors both affiliations landed on one and the other got none.
// 3 dogfood reports across 2 papers; on 2605.22728, 2 of 3 affiliations vanished.

#[test]
fn each_affil_attaches_to_the_author_that_declares_it() {
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\author[1]{Alice}
\author[2]{Bob}
\affil[1]{First Place}
\affil[2]{Second Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(t.contains("First Place"), "affil[1] survives; got:\n{t}");
    assert!(t.contains("Second Place"), "affil[2] survives; got:\n{t}");
    assert!(t.contains("Alice#super[1]"), "Alice keeps her index; got:\n{t}");
    assert!(t.contains("Bob#super[2]"), "Bob keeps his index; got:\n{t}");
}

#[test]
fn an_author_may_declare_several_affiliations() {
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\author[1]{Alice}
\author[2,3]{Bob}
\affil[1]{First Place}
\affil[2]{Second Place}
\affil[3]{Third Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(t.contains("Bob#super[2,3]"), "both indices kept; got:\n{t}");
    for p in ["First Place", "Second Place", "Third Place"] {
        assert!(t.contains(p), "{p} survives; got:\n{t}");
    }
}

// ── Regressions the first attempt introduced (review-verified) ──────────────

#[test]
fn one_raw_author_expanding_to_several_keeps_the_index_aligned() {
    // `parse_authors` splits one `\author{...}` into N on `\and`, so a ref held
    // in a vector parallel to the RAW strings desynchronises: Bob stole Carol's
    // index and Carol lost hers. The ref has to ride on the Author record.
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\author[1]{Alice \and Bob}
\author[2]{Carol}
\affil[1]{First Place}
\affil[2]{Second Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(t.contains("Carol#super[2]"), "Carol keeps index 2; got:\n{t}");
    assert!(
        !t.contains("Bob#super[2]"),
        "Bob must not take Carol's index; got:\n{t}"
    );
}

#[test]
fn a_text_derived_affiliation_survives_alongside_a_numbered_one() {
    // Making the numbered footer REPLACE the text-derived one dropped every
    // `\thanks`/`\affiliation` entry while its author still printed a
    // superscript — Carol was falsely attributed to Alice's institution, which
    // is worse than the bug being fixed.
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\author[1]{Alice}
\author{Carol\thanks{Carol Dept}}
\affil[1]{First Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(t.contains("First Place"), "numbered affil kept; got:\n{t}");
    assert!(t.contains("Carol Dept"), "text-derived affil kept; got:\n{t}");
    assert!(
        !t.contains("Carol#super[1]"),
        "Carol must not be attributed to Alice's institution; got:\n{t}"
    );
}

#[test]
fn a_macro_expanded_author_does_not_shift_the_others() {
    // The refs are positionally parallel to `raw_authors`; a path that moves one
    // without the other shifts every later author onto the wrong institution.
    // This is the desync the record-carried ref was supposed to make impossible.
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\newcommand{\theauthors}{\author[9]{Zed}}
\theauthors
\author[1]{Alice}
\author[2]{Bob}
\affil[1]{First Place}
\affil[2]{Second Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(
        !t.contains("Alice#super[9]"),
        "Alice must not inherit a macro-author's ref; got:\n{t}"
    );
    assert!(
        t.contains("Alice#super[1]") || !t.contains("Alice#super["),
        "Alice keeps her own index or none, never someone else's; got:\n{t}"
    );
}

#[test]
fn a_non_numeric_index_does_not_break_the_document() {
    // `#super[{refs}]` injected the raw optional argument unescaped, so
    // `\author[*]{Alice}` produced an uncompilable document.
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\author[*]{Alice}
\affil[1]{First Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(
        !t.contains("#super[*]"),
        "a non-numeric ref must not reach the output raw; got:\n{t}"
    );
}

#[test]
fn an_orphan_numbered_affil_is_still_reported() {
    // With a declared `[n]` and no author context the handler returned early,
    // skipping the class_metadata capture AND its warning — content vanished
    // with no diagnostic.
    let out = convert(
        r"\documentclass{article}\usepackage{authblk}
\affil[1]{Orphan Place}
\title{T}
\begin{document}\maketitle\end{document}",
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    );
    assert!(
        out.typst.contains("Orphan Place") || !out.warnings.is_empty(),
        "an orphan affil is kept or warned about, not silently dropped; got:\n{}",
        out.typst
    );
}

// ── Regressions found in the second review ─────────────────────────────────

#[test]
fn a_non_numeric_ref_is_rejected_not_rewritten() {
    // Filtering the non-digits out of `a2` yields `2` and files the author under
    // the SECOND institution — a confident wrong answer, worse than none.
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\author[a2]{Alice}
\affil[1]{First Place}
\affil[2]{Second Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(
        !t.contains("Alice#super[2]"),
        "`a2` is not a reference to affiliation 2; got:\n{t}"
    );
}

#[test]
fn a_rejected_ref_keeps_the_authors_own_affiliation() {
    // Suppressing on the RAW ref while emitting from the sanitized one dropped
    // the author's text affiliation AND gave no superscript, losing it entirely.
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\author[1]{Alice}
\author[*]{Carol\thanks{Carol Dept}}
\affil[1]{First Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(t.contains("Carol Dept"), "Carol's affiliation survives; got:\n{t}");
}

#[test]
fn a_declared_ref_without_a_numbered_table_is_ignored() {
    // Otherwise the superscript points at a number the footer never prints.
    let t = typ(
        r"\documentclass{article}
\author[2]{Alice\thanks{MIT}}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(t.contains("MIT"), "the affiliation renders; got:\n{t}");
    assert!(
        !t.contains("#super[2]"),
        "no `#super[2]` without an affil[2] to point at; got:\n{t}"
    );
}

#[test]
fn a_numbered_email_is_not_filed_as_an_affiliation() {
    // The numbered branch sits in an arm shared with `\email`/`\orcid`/…; only
    // the affiliation commands take a numbered slot.
    let t = typ(
        r"\documentclass{article}\usepackage{authblk}
\author[1]{Alice\email[1]{a@x.com}}
\affil[1]{First Place}
\title{T}
\begin{document}\maketitle\end{document}",
    );
    assert!(t.contains("First Place"), "the real affiliation renders; got:\n{t}");
    assert!(
        !t.contains("#super[1] a\\@x.com") && !t.contains("#super[1] a@x.com"),
        "an email is not an affiliation line; got:\n{t}"
    );
}
