//! A `\newenvironment`/`\newcommand` whose definition body contains a `\begin{}`
//! makes tree-sitter wrap the whole thing in an ERROR node (the inner begin has
//! no matching end inside the brace), so the definition was emitted as raw text
//! ("\newenvironment{smallbmatrix}{\left[\begin{smallmatrix}…") into the body.
//! Drop these ERROR-wrapped definitions. Found by the visual grader on 2605.22485.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn newenvironment_with_begin_body_dropped() {
    let t = typ(r"A\newenvironment{smallbmatrix}{\left[\begin{smallmatrix}}{\end{smallmatrix}\right]}B");
    assert!(!t.contains("newenvironment"), "newenvironment leaked; got:\n{t}");
    assert!(!t.contains("smallbmatrix"), "definition body leaked; got:\n{t}");
}

#[test]
fn newcommand_with_begin_body_dropped() {
    let t = typ(r"X\newcommand{\mat}{\begin{smallmatrix}a\end{smallmatrix}}Y");
    assert!(!t.contains("newcommand"), "newcommand leaked; got:\n{t}");
}
