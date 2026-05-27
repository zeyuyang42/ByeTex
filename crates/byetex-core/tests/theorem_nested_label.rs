//! Regression tests for labels nested inside theorem-like environments.
//!
//! `\begin{theorem} content\n\begin{enumerate}...\end{enumerate}\label{X}\end{theorem}`
//! — when `\label{X}` is inside a nested environment (e.g. enumerate) rather
//! than a direct child of the theorem env, the label used to end up inside
//! the `#figure(kind: "theorem", ...)` content as a text label, causing
//! Typst's "cannot reference text" error (arXiv:2605.22724).

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// `\label` at the very end of a theorem body, after a nested enumerate.
/// The label should be hoisted outside the `#figure()` call.
#[test]
fn label_at_end_of_nested_enumerate_in_theorem() {
    let src = r"
\begin{document}
\newtheorem{assumption}{Assumption}
\begin{assumption}
We make the following assumptions.
\begin{enumerate}
\item First assumption.
\item Second assumption with $a \leq b$.
\end{enumerate}\label{ass:main}
\end{assumption}
Recall Assumption \ref{ass:main} above.
\end{document}
";
    let t = typst(src);

    // The label must appear OUTSIDE the figure content (after the closing ')').
    // Pattern: "#figure(...) <ass:main>" — label after the figure call.
    assert!(
        t.contains("<ass:main>"),
        "label not found in output;\noutput:\n{t}"
    );
    // The label must NOT appear inside the figure content (between the '[' and ']'
    // of the figure body).  If it does, Typst will abort with "cannot reference text".
    let figure_start = t.find("#figure(").unwrap_or(0);
    let label_pos = t.find("<ass:main>").unwrap_or(usize::MAX);
    // Find the closing ')' of the figure call (approximate: the last ')' before the label).
    let figure_body_close = t[figure_start..label_pos].rfind(')').map(|i| figure_start + i);
    assert!(
        figure_body_close.is_some() && label_pos > figure_body_close.unwrap(),
        "label is inside the #figure() content — will cause 'cannot reference text';\noutput:\n{t}"
    );
}

/// `\label` as a direct child of the theorem env (first child after begin): must still work.
/// This is the existing fast-path — must not regress.
#[test]
fn label_as_direct_child_of_theorem_unchanged() {
    let src = r"
\begin{document}
\newtheorem{theorem}{Theorem}
\begin{theorem}
\label{thm:direct}
Statement of the theorem.
\end{theorem}
By Theorem \ref{thm:direct}.
\end{document}
";
    let t = typst(src);

    assert!(
        t.contains("<thm:direct>"),
        "label missing;\noutput:\n{t}"
    );
    let figure_start = t.find("#figure(").unwrap_or(0);
    let label_pos = t.find("<thm:direct>").unwrap_or(usize::MAX);
    let figure_body_close = t[figure_start..label_pos].rfind(')').map(|i| figure_start + i);
    assert!(
        figure_body_close.is_some() && label_pos > figure_body_close.unwrap(),
        "label is inside the figure — regression;\noutput:\n{t}"
    );
}

/// `\label` at the end of theorem body as the last line (direct child, sibling of text).
#[test]
fn label_at_end_of_theorem_body() {
    let src = r"
\begin{document}
\newtheorem{lemma}{Lemma}
\begin{lemma}
Statement of the lemma.
\label{lem:end}
\end{lemma}
By Lemma \ref{lem:end}.
\end{document}
";
    let t = typst(src);

    assert!(
        t.contains("<lem:end>"),
        "label missing;\noutput:\n{t}"
    );
    let figure_start = t.find("#figure(").unwrap_or(0);
    let label_pos = t.find("<lem:end>").unwrap_or(usize::MAX);
    let figure_body_close = t[figure_start..label_pos].rfind(')').map(|i| figure_start + i);
    assert!(
        figure_body_close.is_some() && label_pos > figure_body_close.unwrap(),
        "label at end of body not hoisted outside figure;\noutput:\n{t}"
    );
}
