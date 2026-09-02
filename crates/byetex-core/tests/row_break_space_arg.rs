//! `\\ [1ex]` — a row break whose vertical-space argument is separated from it by
//! a SPACE — leaked its argument into the next cell as literal `[\[1ex\]` text.
//!
//! `\\[1ex]` (glued) was already consumed; the spaced form is equally valid LaTeX
//! (TeX skips spaces before an optional argument) and is what booktabs tables in
//! the wild often carry. 22 occurrences across 6 corpus papers; found by a dogfood
//! agent on gh-amberj-latex-book-template.

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
fn a_spaced_row_break_argument_is_consumed() {
    let t = typ("\\begin{tabular}{ll}\na & b \\\\ [1ex]\nc & d \\\\\n\\end{tabular}");
    assert!(!t.contains("1ex"), "the spacing hint must not render; got:\n{t}");
    assert!(t.contains("[a], [b]"), "first row intact; got:\n{t}");
    assert!(t.contains("[c], [d]"), "second row intact; got:\n{t}");
}

#[test]
fn the_glued_form_still_works() {
    // The control for the change: `\\[0.5ex]` already worked and must keep working.
    let t = typ("\\begin{tabular}{ll}\na & b \\\\[0.5ex]\nc & d \\\\\n\\end{tabular}");
    assert!(!t.contains("0.5ex"), "glued form still consumed; got:\n{t}");
    assert!(t.contains("[c], [d]"), "rows still split; got:\n{t}");
}

#[test]
fn every_length_unit_is_recognised() {
    for unit in ["ex", "em", "pt", "mm", "cm", "in"] {
        let src = format!("\\begin{{tabular}}{{ll}}\na & b \\\\ [2{unit}]\nc & d \\\\\n\\end{{tabular}}");
        let t = typ(&src);
        assert!(!t.contains(&format!("2{unit}")), "2{unit} consumed; got:\n{t}");
    }
}

#[test]
fn a_negative_length_is_recognised() {
    let t = typ("\\begin{tabular}{ll}\na & b \\\\ [-2pt]\nc & d \\\\\n\\end{tabular}");
    assert!(!t.contains("-2pt"), "a negative hint is still a hint; got:\n{t}");
}

#[test]
fn the_spaced_form_matches_the_glued_form_exactly() {
    // `\\` takes an optional LENGTH in LaTeX, and TeX skips spaces before an
    // optional argument — so `\\ [1ex]` and `\\[1ex]` are the same construct and
    // must convert identically. An earlier draft of this fix accepted only
    // number+unit for the spaced form, which left the two paths disagreeing:
    // `\\[see note]` dropped its bracket while `\\ [see note]` kept it.
    let spaced = typ("\\begin{tabular}{ll}\na & b \\\\ [1ex]\nc & d \\\\\n\\end{tabular}");
    let glued = typ("\\begin{tabular}{ll}\na & b \\\\[1ex]\nc & d \\\\\n\\end{tabular}");
    assert_eq!(
        spaced, glued,
        "the spaced and glued forms are the same LaTeX and must convert alike"
    );
}

#[test]
fn a_minipage_row_break_argument_is_consumed_too() {
    // The fix belongs at the text-mode `\\` arm, not in the table row splitter:
    // a `minipage` emits `#linebreak()` and never reaches that splitter, so a
    // table-only fix could not have covered it.
    let t = typ(r"\begin{minipage}{5cm} a \\ [1ex] b \end{minipage}");
    assert!(!t.contains("1ex"), "minipage row break consumes its length; got:\n{t}");
    assert!(t.contains("#linebreak()"), "the break still renders; got:\n{t}");
}
