//! Regression: a `\newcommand` body where a control word is immediately
//! followed by a parameter (`\langle#1`) must keep the control-word token
//! boundary when the argument is substituted.
//!
//! Root cause: `substitute_macro_args` did a raw string splice, so
//! `\langle#1` with `#1 = do` produced `\langledo` (one long, unknown control
//! word) instead of `\langle do` (`\langle` + the text `do`). The math symbol
//! lookup then never fired and `langledo` leaked as a literal math string.
//! In TeX, `\langle` is a complete control word terminated by the non-letter
//! `#`, so the expansion is `\langle` followed by the argument tokens.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn control_word_before_param_keeps_boundary() {
    // \tokenstring{do} = \langle do \rangle ; must map \langle -> chevron.l.
    let src = r"\begin{document}
\newcommand{\tokenstring}[1]{\langle#1\rangle}
$\tokenstring{do}$
\end{document}";
    let t = typst(src);
    assert!(
        t.contains("chevron.l"),
        "`\\langle` did not survive expansion (expected chevron.l); output:\n{t}"
    );
    assert!(
        !t.contains("langledo") && !t.contains("langled"),
        "control word merged with the argument (`langledo`); output:\n{t}"
    );
}

#[test]
fn ordinary_letters_before_param_still_concatenate() {
    // `my#1` is ordinary text, NOT a control word — it must keep concatenating
    // (the boundary is only inserted after a `\`-led control word).
    let src = r"\begin{document}
\newcommand{\pfx}[1]{my#1}
Here is \pfx{word} done.
\end{document}";
    let t = typst(src);
    assert!(
        t.contains("myword"),
        "ordinary-letter concatenation broke (`myword` missing); output:\n{t}"
    );
}

#[test]
fn control_word_before_nonletter_param_unaffected() {
    // A digit-starting arg already terminates the control word — no boundary
    // needed, and none should be spuriously added that changes the symbol.
    let src = r"\begin{document}
\newcommand{\sym}[1]{\alpha#1}
$\sym{2}$
\end{document}";
    let t = typst(src);
    assert!(
        t.contains("alpha"),
        "`\\alpha` did not survive expansion; output:\n{t}"
    );
}
