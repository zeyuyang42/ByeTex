//! Expanded-corpus compile-blocker (2605.31567): `\textit{correct }transformation`
//! — a trailing space INSIDE the emphasis braces. byetex dropped it and emitted
//! `_correct_transformation`; Typst's `_` emphasis shorthand requires a word
//! boundary at the closing marker, so the opening `_` never closes → `unclosed
//! delimiter`. The surrounding whitespace must sit OUTSIDE the markers
//! (`_correct_ transformation`).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn textit_trailing_space_moves_outside_markers() {
    let t = typ("the \\textit{correct }transformation is known");
    // The space must be AFTER the closing `_`, not before it (and not dropped).
    assert!(
        t.contains("_correct_ transformation"),
        "trailing space must sit outside the emphasis markers; got:\n{t}"
    );
    assert!(
        !t.contains("_correct_transformation") && !t.contains("correct _"),
        "no glued/leading-space closing marker; got:\n{t}"
    );
}

#[test]
fn textbf_leading_space_moves_outside_markers() {
    // `a\textbf{ bold}word`: the internal leading space moves OUT of the markers
    // (no `* bold`), and because the closing side is glued to `word`, the
    // boundary-safe function form is used: `a #strong[bold]word`. The old
    // `a *bold*word` shorthand left the bold unclosed in Typst (see
    // emphasis_word_boundary.rs).
    let t = typ("a\\textbf{ bold}word");
    assert!(
        t.contains("a #strong[bold]word"),
        "leading space outside markers + glued close uses fn form; got:\n{t}"
    );
    assert!(
        !t.contains("* bold") && !t.contains("[ bold"),
        "no space after the opening marker; got:\n{t}"
    );
}

#[test]
fn emph_no_surrounding_space_unchanged() {
    // Regression guard: the clean case keeps the tight `_word_` form.
    let t = typ("an \\emph{italic} word");
    assert!(
        t.contains("_italic_"),
        "clean emphasis must stay `_italic_`; got:\n{t}"
    );
}
