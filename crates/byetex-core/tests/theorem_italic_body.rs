//! amsthm's `plain` style (the default) sets the theorem BODY in italic
//! (Theorem/Lemma/Proposition/…); the `definition` and `remark` styles set it
//! upright. ByeTex rendered every theorem body upright. Track `\theoremstyle`
//! in document order and emphasize only plain-style bodies — and only when
//! amsthm is loaded (base-LaTeX `\newtheorem` is upright). Found by the visual
//! grader on 2605.22159.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const DOC: &str = r"\documentclass{article}\usepackage{amsthm}
\theoremstyle{plain}\newtheorem{theorem}{Theorem}
\theoremstyle{definition}\newtheorem{definition}{Definition}
\begin{document}
\begin{theorem}A plain theorem body.\end{theorem}
\begin{definition}A definition body.\end{definition}
\end{document}";

#[test]
fn plain_theorem_body_is_italic() {
    let t = typ(DOC);
    // The plain-style theorem show rule emphasizes the body.
    let thm_rule = t
        .lines()
        .find(|l| l.contains("figure.where(kind: \"theorem\")"))
        .unwrap_or_else(|| panic!("no theorem show rule; got:\n{t}"));
    assert!(
        thm_rule.contains("emph[#it.body]"),
        "plain theorem body not italicized; rule:\n{thm_rule}"
    );
}

#[test]
fn definition_body_stays_upright() {
    let t = typ(DOC);
    let def_rule = t
        .lines()
        .find(|l| l.contains("figure.where(kind: \"definition\")"))
        .unwrap_or_else(|| panic!("no definition show rule; got:\n{t}"));
    assert!(
        !def_rule.contains("emph[#it.body]"),
        "definition body should be upright (\\theoremstyle{{definition}}); rule:\n{def_rule}"
    );
}

#[test]
fn no_amsthm_keeps_upright() {
    // Base-LaTeX \newtheorem (no amsthm) → upright body.
    let t = typ(r"\documentclass{article}\newtheorem{theorem}{Theorem}\begin{document}\begin{theorem}Body.\end{theorem}\end{document}");
    let rule = t.lines().find(|l| l.contains("figure.where(kind: \"theorem\")"));
    if let Some(r) = rule {
        assert!(!r.contains("emph[#it.body]"), "no-amsthm theorem should stay upright; rule:\n{r}");
    }
}
