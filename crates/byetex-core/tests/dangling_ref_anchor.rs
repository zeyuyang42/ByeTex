//! Expanded-corpus compile-blocker (2605.31586): `\cref{ex:nonarbitrary_fo2}`
//! references a `\label` that is COMMENTED OUT. LaTeX treats this as a soft
//! "undefined reference" warning and still compiles; byetex emits a bare
//! `@key`, and Typst aborts: `label <key> does not exist`.
//!
//! finish() now backstops this: any `\ref`/`\cref`-referenced key (tracked in
//! `referenced_labels`) with neither a `<key>` label in the output nor a
//! bibliography entry gets a hidden anchor so the reference resolves and the
//! document compiles (matching LaTeX's compile-anyway behaviour). Scoped to the
//! ref family — citation keys (resolved by `#bibliography`) are untouched, so
//! the backstop can never collide with the bibliography.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// Wrap a body in a minimal full document so the backstop (document-only)
/// runs — bare fragments are intentionally left untouched.
fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn ref_to_undefined_label_gets_backstop_anchor() {
    // `\cref{ghost}` with no `\label{ghost}` anywhere.
    let t = typ(&doc("See \\cref{ghost} for details."));
    assert!(
        t.contains("@ghost"),
        "the reference must be emitted; got:\n{t}"
    );
    assert!(
        t.contains("<ghost>"),
        "an undefined reference must get a backstop anchor so it resolves; got:\n{t}"
    );
}

#[test]
fn ref_to_defined_label_gets_no_duplicate_anchor() {
    // Regression guard: a real label must NOT get a second backstop anchor
    // (which would itself be a duplicate-label error).
    let t = typ(&doc(
        "\\section{Intro}\\label{sec:intro}\nSee \\cref{sec:intro}.",
    ));
    let n = t.matches("<sec:intro>").count();
    assert_eq!(
        n, 1,
        "a defined label must appear exactly once (no backstop dup); got {n}:\n{t}"
    );
}

#[test]
fn email_at_is_not_mistaken_for_a_reference() {
    // Regression guard: `a@b.com` is escaped to `\@` and must NOT be treated as
    // a reference needing an anchor.
    let t = typ(&doc("Contact me at user@example.com today."));
    assert!(
        !t.contains("<example.com>") && !t.contains("<com>"),
        "an email `@` must not spawn a backstop anchor; got:\n{t}"
    );
}

#[test]
fn bare_fragment_gets_no_backstop_anchor() {
    // A bare fragment (no \documentclass) may be embedded where the label is
    // defined — it must NOT get a backstop anchor that would then collide.
    let t = typ("See \\cref{ghost} for details.\n");
    assert!(
        !t.contains("<ghost>"),
        "a bare fragment must not get a backstop anchor; got:\n{t}"
    );
}
