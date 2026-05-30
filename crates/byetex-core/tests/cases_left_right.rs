//! Regression tests for `\left`/`\right` delimiters inside `\begin{cases}`.
//!
//! When `\left(\frac{...}\right)` spans a row inside a `cases` environment,
//! the `\right)` must be emitted as `)` (plain paren), not as `\)`.
//! Emitting `\)` caused Typst to treat it as a separate unclosed delimiter
//! and abort with "unclosed delimiter" (arXiv:2605.22765).

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// `\left(...\right)` inside a cases row must not emit `\)`.
/// Correct output should have plain `)` not `\)` after the content.
#[test]
fn left_right_in_cases_no_backslash_paren() {
    let src = r"\begin{document}
\[
\begin{cases}
    f(x) & x > 0, \\
    g\!\left(\frac{a+b}{c}\right), & x = 0,
\end{cases}
\]
\end{document}";
    let t = typst(src);

    // The `\right)` must become `)` — no `\)` in the cases output.
    assert!(
        !t.contains("\\)"),
        "\\) found in output — \\right) emitted raw;\noutput:\n{t}"
    );
    // The output must contain a valid cases(...) call.
    assert!(
        t.contains("cases("),
        "cases() call missing from output;\noutput:\n{t}"
    );
}

/// Multi-row cases with `\left...\right` wrapping a fraction in the second row.
/// Neither row must produce `\)` in the Typst output.
#[test]
fn left_right_frac_in_cases_second_row() {
    let src = r"\begin{document}
\[
p(x) = \begin{cases}
    \alpha(x; \mu) & x \neq x_0, \\
    \alpha\!\left(\frac{\beta + (1-\beta)x_0}{1+\gamma}\right), & x = x_0,
\end{cases}
\]
\end{document}";
    let t = typst(src);

    assert!(!t.contains("\\)"), "\\) found in output;\noutput:\n{t}");
    assert!(t.contains("cases("), "cases() call missing;\noutput:\n{t}");
}

/// Existing case: `\left...\right` NOT inside cases must still work.
/// This ensures the fix doesn't regress regular math mode.
#[test]
fn left_right_outside_cases_unchanged() {
    let src = r"\begin{document}
\[ \left(\frac{a}{b}\right) \]
\end{document}";
    let t = typst(src);

    // Should have a fraction — `(a) / (b)` — and NO `\)`.
    assert!(
        !t.contains("\\)"),
        "\\) found outside cases context;\noutput:\n{t}"
    );
    assert!(t.contains("/ (b)"), "fraction not rendered;\noutput:\n{t}");
}

/// Stray `)` inside `\frac` numerator inside `\left...\right` inside `cases`.
/// The stray `)` in `\frac{\alpha x_0)}{...}` must not confuse the bracket
/// balancer into escaping the closing `)` of the `cases(...)` call itself —
/// which would make it unclosable and trigger Typst's "unclosed delimiter".
#[test]
fn stray_paren_in_frac_numerator_inside_left_right_in_cases() {
    let src = r"\begin{document}
\begin{equation}
\begin{cases}
    f(x_s; x_s) & x_s \neq x_0, \\
    g\!\left(x_s;\frac{\alpha x_0)}{1+\alpha}\right), & x_s = x_0,
\end{cases}
\end{equation}
\end{document}";
    let t = typst(src);

    assert!(t.contains("cases("), "cases() missing;\noutput:\n{t}");
    // The closing `)` of `cases(...)` must NOT be escaped to `\)`.
    let cases_start = t.find("cases(").expect("cases( not found");
    let after_cases = &t[cases_start..];
    assert!(
        !after_cases.contains("]\\ $") && !after_cases.contains("]\\)"),
        "cases closing ) was escaped to \\) — stray paren leaked outside [...];\noutput:\n{t}"
    );
}
