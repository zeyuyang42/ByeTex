//! LaTeX `\\*` is the no-page-break variant of the `\\` forced line break, and
//! `\\[len]` carries an optional vertical skip. The `*` (and `[len]`) have no
//! Typst analog and MUST be consumed. Emitted literally, the `*` is read by
//! Typst as a strong-emphasis toggle, so a run of `\\*` lines (verse/poetry)
//! unbalances `*` and yields `error: unclosed delimiter` — reported far
//! downstream from the real culprit (corpus ctan-memoir: the Villon ballade
//! refrain, "...do not ask in a week \\*").

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn starred_linebreak_consumes_the_star() {
    // Three `\\*` line breaks; the stars must not survive as emphasis toggles.
    let t = typ(r"Prince \\* year \\* refrain \\* yesteryear?");
    assert!(
        !t.contains('*'),
        "the `*` after `\\\\` must be consumed (no stray strong toggle); got:\n{t}"
    );
}

#[test]
fn starred_linebreak_still_breaks_the_line() {
    // It is still a line break — the surrounding words stay, only the `*` goes.
    let t = typ(r"first line \\* second line");
    assert!(t.contains("first line"), "got:\n{t}");
    assert!(t.contains("second line"), "got:\n{t}");
    assert!(!t.contains("\\* second"), "no leaked star before the next line; got:\n{t}");
}

#[test]
fn starred_linebreak_with_length_consumes_both() {
    // `\\*[1ex]` — star AND optional length must both be consumed.
    let t = typ(r"one \\*[1ex] two");
    assert!(!t.contains('*'), "star consumed; got:\n{t}");
    assert!(!t.contains("[1ex]") && !t.contains("1ex"), "length consumed; got:\n{t}");
}
