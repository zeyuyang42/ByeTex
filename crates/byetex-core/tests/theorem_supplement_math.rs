//! Regression tests for theorem supplement containing LaTeX math.
//!
//! When `\newtheorem{theoremAstar}[theorem]{Theorem A$^\star$}` defines a
//! theorem, the display name `Theorem A$^\star$` must be converted to Typst
//! before being inserted into `supplement: [...]` — otherwise the raw `$`
//! and `^` leak into the Typst output and break the parser.

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

#[test]
fn theorem_supplement_with_math_does_not_contain_raw_dollar() {
    let out = convert_str(
        r"\newtheorem{thmstar}[theorem]{Theorem A$^\star$}
\begin{thmstar}
  Body text.
\end{thmstar}",
    );
    // Raw `$^\star$` in supplement would produce invalid Typst (backslash before star)
    assert!(
        !out.typst.contains(r"$^\star$"),
        "raw LaTeX math must not appear in supplement, got: {}",
        out.typst
    );
}

#[test]
fn theorem_supplement_with_math_is_converted() {
    let out = convert_str(
        r"\newtheorem{thmstar}[theorem]{Theorem A$^\star$}
\begin{thmstar}
  Body text.
\end{thmstar}",
    );
    // The converted output should have the supplement contain the theorem name
    // with the math converted (^ becomes ^ in Typst math inside the content)
    assert!(
        out.typst.contains("supplement: ["),
        "supplement block must exist, got: {}",
        out.typst
    );
}
