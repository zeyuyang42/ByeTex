//! Expanded-corpus compile-blocker (2605.31567): a bare `<` in a TEXT table
//! cell (`& <0.001 &`, a p-value) was escaped TWICE — once by
//! `escape_text_cell` and again by the whole-output `post_process_typography`
//! pass — producing `\\<0.001` (a literal backslash-backslash followed by an
//! UNescaped `<`). Typst then read the bare `<` as the start of a label and
//! aborted with `unclosed label`.
//!
//! `post_process_typography` is the single, math-aware owner of `<` escaping
//! (it leaves a real `<key>` label alone and escapes everything else), so
//! `escape_text_cell` must NOT also escape `<`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn bare_less_than_in_text_cell_is_escaped_once() {
    let t = typ("\\begin{tabular}{cc}\nA & <0.001 \\\\\n\\end{tabular}\n");
    // Exactly one backslash before the `<` — not the double-escaped `\\<`.
    assert!(
        t.contains("\\<0.001") && !t.contains("\\\\<0.001"),
        "`<` in a text cell must be escaped once, not doubled; got:\n{t}"
    );
}

#[test]
fn less_than_in_math_cell_unaffected() {
    // Regression guard: `<` inside a cell's `$...$` math is handled by
    // post_process (math-aware) and must not be double-escaped either.
    let t = typ("\\begin{tabular}{cc}\nA & $a<b$ \\\\\n\\end{tabular}\n");
    assert!(
        !t.contains("\\\\<"),
        "math-cell `<` must not be double-escaped; got:\n{t}"
    );
}
