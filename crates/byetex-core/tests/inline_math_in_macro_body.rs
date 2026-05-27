//! Regression tests: `\newcommand` bodies that contain `$...$` must not
//! produce nested `$...$` delimiters when the macro is expanded inside math.
//!
//! Root cause: `emit_inline_math` was emitting `$...$` unconditionally, so
//! a macro body like `$\mid$` expanded inside `$...$` produced `$divides$` —
//! a `$` sign inside Typst math causes "unclosed delimiter" errors.
//!
//! Fix: add an `in_math` guard to `emit_inline_math`: when already inside a
//! math container, emit the body children directly without `$` wrappers.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// Simplest case: `\newcommand{\mybar}{$\mid$}` used in math.
/// Must produce `divides` without surrounding `$`, not `$divides$`.
#[test]
fn macro_with_dollar_mid_in_math_no_nested_dollar() {
    let src = r"\begin{document}
\newcommand{\mybar}{$\mid$}
$a \mybar b$
\end{document}";
    let t = typst(src);
    assert!(
        !t.contains("$divides$"),
        "`$divides$` found — nested `$...$` in math; output:\n{t}"
    );
    assert!(
        t.contains("divides"),
        "`divides` not found in output; output:\n{t}"
    );
}

/// Realistic case: `\tbar` from 2605.22765 corpus paper.
/// `\tbar` = `\mathrel{\raisebox{0.15ex}{$\scriptscriptstyle\mid$}}`.
/// Used as a subscript inside math: `alpha_(t\tbar s)`.
#[test]
fn tbar_macro_in_math_subscript_no_nested_dollar() {
    let src = r"\begin{document}
\newcommand{\tbar}{\mathrel{\raisebox{0.15ex}{$\scriptscriptstyle\mid$}}}
$\alpha_{t\tbar s}$
\end{document}";
    let t = typst(src);
    assert!(
        !t.contains("$divides$"),
        "`$divides$` found — nested `$...$` in math subscript; output:\n{t}"
    );
    // `divides` should appear (from \mid) but NOT wrapped in its own $...$
    assert!(
        t.contains("divides"),
        "`divides` not in output at all; output:\n{t}"
    );
}

/// `$...$` inside a macro body used in text mode must still produce `$...$`
/// in the output (the guard must only suppress the extra dollars in math mode).
#[test]
fn macro_with_dollar_in_text_mode_keeps_dollars() {
    let src = r"\begin{document}
\newcommand{\mybar}{$\mid$}
Here is \mybar{} in text.
\end{document}";
    let t = typst(src);
    // In text mode the expansion should produce `$divides$` (inline math block).
    assert!(
        t.contains("$divides$"),
        "`$divides$` (inline math) not found for text-mode expansion; output:\n{t}"
    );
}
