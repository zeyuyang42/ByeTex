//! `\let\new\old` should register `\new` as an alias of `\old`, not emit
//! spurious "unsupported command" warnings. Tree-sitter parses it as a
//! dedicated `let_command_definition` node (declaration + implementation),
//! and handles the `\let\new=\old` form identically.

use byetex_core::{convert, Category, ConvertOptions};

fn out(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

fn has_unsupported(src: &str, cmd: &str) -> bool {
    out(src)
        .warnings
        .iter()
        .any(|w| matches!(&w.category, Category::UnsupportedCommand { name } if name == cmd))
}

#[test]
fn let_aliases_a_builtin_symbol() {
    // \myrel (not a builtin) aliased to \leq, then used in math.
    let t = out("\\let\\myrel\\leq\n$a \\myrel b$").typst;
    assert!(
        t.contains("<="),
        "\\myrel should alias \\leq -> <=; got:\n{t}"
    );
}

#[test]
fn let_eq_form_aliases_too() {
    // The `\let\new=\old` form parses to the same node; must behave the same.
    let t = out("\\let\\myrel=\\leq\n$a \\myrel b$").typst;
    assert!(
        t.contains("<="),
        "\\let\\myrel=\\leq should alias -> <=; got:\n{t}"
    );
}

#[test]
fn let_copies_a_user_macro() {
    let t = out("\\newcommand{\\foo}{FOO}\n\\let\\bar\\foo\n\\bar").typst;
    assert!(
        t.contains("FOO"),
        "\\bar should alias \\foo -> FOO; got:\n{t}"
    );
}

#[test]
fn let_does_not_warn_unsupported() {
    // Neither \let itself nor the freshly-aliased name should warn.
    assert!(
        !has_unsupported("\\let\\myrel\\leq\n$a \\myrel b$", "\\let"),
        "\\let must not warn"
    );
    assert!(
        !has_unsupported("\\let\\myrel\\leq\n$a \\myrel b$", "\\myrel"),
        "the aliased \\myrel must not warn as unsupported"
    );
}

#[test]
fn let_does_not_emit_the_operand_names_as_text() {
    // The definition itself produces no visible output (no stray \foo / \bar).
    let t = out("\\let\\bar\\leq").typst;
    assert!(
        !t.contains("bar") && !t.contains("\\let"),
        "a bare \\let definition should emit nothing; got:\n{t}"
    );
}
