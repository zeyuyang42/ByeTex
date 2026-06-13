//! A function-form inline wrap (`#smallcaps[...]`, `#strong[...]`, …) directly
//! followed by `(` is parsed by Typst as a call chain — `#smallcaps[X](Y)` reads
//! `(Y)` as an argument list, and an inner `#raw(...)` then has an invalid `#`
//! ("the character # is not valid in code"). Source `\textsc{X}(\texttt{y})`
//! produced `#smallcaps[X](#raw("y"))` (corpus 2605.31584).
//!
//! Fix: when a `#f[...]` wrap is immediately followed by `(`, insert a
//! zero-width space so the `(` stays literal markup.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn smallcaps_followed_by_paren_does_not_chain() {
    let t = typst(r"\textsc{LongTraceRL}(\texttt{random})");
    assert!(
        !t.contains("#smallcaps[LongTraceRL](#raw"),
        "the function wrap must not be directly glued to `(`;\noutput:\n{t:?}"
    );
    assert!(
        t.contains("#smallcaps[LongTraceRL]\u{200B}("),
        "expected a zero-width-space break between `]` and `(`;\noutput:\n{t:?}"
    );
}

#[test]
fn wrap_not_followed_by_paren_unchanged() {
    let t = typst(r"\textsc{ABC} and more");
    assert!(
        !t.contains('\u{200B}'),
        "no break inserted when `(` does not follow;\noutput:\n{t:?}"
    );
}
