//! TeX register/penalty/dimen assignments (`\clubpenalty=300`,
//! `\interfootnotelinepenalty=10000`, `\parskip=0pt plus 1pt`) are preamble-tuning
//! primitives with no visual content. The grammar parses the command alone and
//! left the `=<value>` tail as sibling tokens that leaked into the body as text
//! (e.g. `=10000` rendered as a stray heading — corpus 2605.31586, dogfood
//! backlog F4). The command is now dropped together with its assignment tail.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn body(src: &str) -> String {
    typ(&format!(
        "\\documentclass{{article}}\\begin{{document}}\n{src}\nKeep this body.\n\\end{{document}}"
    ))
}

#[test]
fn penalty_assignment_tail_does_not_leak() {
    let t = body(r"\interfootnotelinepenalty=10000");
    assert!(!t.contains("10000"), "assignment value must not leak; got:\n{t}");
    assert!(t.contains("Keep this body."), "body must survive; got:\n{t}");
}

#[test]
fn small_penalty_assignment_tail_does_not_leak() {
    let t = body(r"\clubpenalty=300");
    assert!(!t.contains("=300") && !t.contains("\n300"), "value must not leak; got:\n{t}");
}

#[test]
fn dimen_assignment_with_unit_does_not_leak() {
    // Use a distinctive value that can't collide with the neutral preamble's
    // own lengths (e.g. `1.2em`).
    let t = body(r"\parindent=17pt");
    assert!(!t.contains("17pt"), "dimen must not leak; got:\n{t}");
    assert!(t.contains("Keep this body."), "body must survive; got:\n{t}");
}

#[test]
fn glue_assignment_with_plus_minus_does_not_leak() {
    let t = body(r"\parskip=0pt plus 1pt minus 1pt");
    assert!(!t.contains("plus 1pt"), "glue tail must not leak; got:\n{t}");
    assert!(t.contains("Keep this body."), "body must survive; got:\n{t}");
}

#[test]
fn unrelated_equals_in_text_is_untouched() {
    // Plain body text with `=` (not a command assignment) must be preserved.
    let t = body(r"The ratio $a = b$ holds and x = y too.");
    assert!(t.contains("x = y"), "ordinary text `=` must survive; got:\n{t}");
}
