//! Typst treats `*`/`_` emphasis shorthands as delimiters ONLY at a word
//! boundary: a marker glued between two word characters is a literal asterisk/
//! underscore, not a strong/emph toggle. So `\textbf{N}eural` naively emitted as
//! `*N*eural` leaves the opening `*` unclosed → `error: unclosed delimiter`
//! (corpus 2606.12406, which bolds single letters to define acronyms:
//! "\textbf{N}eural \textbf{Ex}ternal \textbf{T}orque").
//!
//! Fix: when a shorthand marker would glue to an adjacent alphanumeric (on
//! either side), emit the boundary-independent function form instead —
//! `#strong[...]` / `#emph[...]`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn textbf_glued_to_following_word_uses_strong_fn() {
    let t = typ(r"We present \textbf{N}eural networks.");
    assert!(
        t.contains("#strong[N]eural"),
        "glued bold must use function form; got:\n{t}"
    );
    assert!(
        !t.contains("*N*eural"),
        "must not emit the unclosed `*N*eural` shorthand; got:\n{t}"
    );
}

#[test]
fn emph_glued_to_following_word_uses_emph_fn() {
    let t = typ(r"the \emph{x}suffix here");
    assert!(
        t.contains("#emph[x]suffix"),
        "glued emph must use function form; got:\n{t}"
    );
    assert!(!t.contains("_x_suffix"), "got:\n{t}");
}

#[test]
fn textbf_preceded_by_word_uses_strong_fn() {
    // Opening marker glued to a preceding word char also breaks Typst.
    let t = typ(r"pre\textbf{X} done");
    assert!(
        t.contains("pre#strong[X]"),
        "leading-glued bold must use function form; got:\n{t}"
    );
    assert!(!t.contains("pre*X*"), "got:\n{t}");
}

#[test]
fn textbf_at_word_boundary_keeps_shorthand() {
    // Space after the group → closing marker is at a boundary → shorthand is
    // safe and preferred (don't regress the common case into verbose fn form).
    let t = typ(r"a \textbf{bold} word");
    assert!(t.contains("*bold*"), "got:\n{t}");
    assert!(!t.contains("#strong"), "got:\n{t}");
}

#[test]
fn textbf_followed_by_punctuation_keeps_shorthand() {
    // Punctuation is a boundary in Typst: `*bold*.` and `*bold*-ish` are fine.
    let t = typ(r"a \textbf{bold}-ish thing.");
    assert!(t.contains("*bold*-ish"), "got:\n{t}");
    assert!(!t.contains("#strong"), "got:\n{t}");
}

#[test]
fn makecell_internal_linebreak_does_not_glue_markup() {
    // `\makecell{\textbf{A}\\\textbf{B}}`: the intra-cell `\\` must become a
    // `#linebreak()`, not a bare `\` glued to the next `*` (which Typst reads as
    // an escaped literal `\*`, leaving the bold unclosed).
    let t = typ(r"\makecell{\textbf{A}\\\textbf{B}}");
    assert!(
        t.contains("#linebreak()"),
        "makecell line break must be #linebreak(); got:\n{t}"
    );
    assert!(
        !t.contains(r"\*"),
        "no escaped-asterisk glue from a bare linebreak; got:\n{t}"
    );
}
