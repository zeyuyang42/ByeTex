//! Bug (corpus 2605.31549): a superscript star adjacent to a division —
//! `\bar{h}_2^*/\Vert…\Vert` → `..._2^*/||…||` — put `*/` in the output, which
//! Typst's lexer reads as a block-comment CLOSE (`/* … */`), aborting with
//! "unexpected end of block comment". `*/` (and `/*`) are never meaningful
//! adjacent in math, so break them with a space.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn superscript_star_then_slash_is_broken() {
    let t = typ("$h^*/x$");
    assert!(
        !t.contains("*/"),
        "`*/` (Typst comment-close) must be broken in math; got:\n{t}"
    );
    assert!(
        t.contains("h^* /") || t.contains("^* /"),
        "expected `^* /`; got:\n{t}"
    );
}

#[test]
fn slash_then_star_is_broken() {
    let t = typ("$a/*b$");
    assert!(
        !t.contains("/*"),
        "`/*` (comment-open) must be broken in math; got:\n{t}"
    );
}

#[test]
fn star_not_adjacent_to_slash_unchanged() {
    // Regression guard: a superscript star with no following slash is untouched.
    let t = typ("$a^*b$");
    assert!(
        t.contains("a^*b"),
        "unrelated `^*` must be unchanged; got:\n{t}"
    );
}
