//! `\begin{proof}…\end{proof}` (amsthm) ends with a flush-right QED tombstone
//! `□` and, given an optional argument, uses it as the proof title ("Proof of
//! Theorem 1." instead of "Proof."). ByeTex emitted `*Proof.* body` with no
//! tombstone and silently dropped the optional name. Found by the visual grader
//! on 2605.22159 (15+ proofs, every one missing the □).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn proof_ends_with_flush_right_qed() {
    let t = typ(r"\begin{proof}A short proof.\end{proof}");
    assert!(t.contains("*Proof.*"), "no proof label; got:\n{t}");
    // Flush-right open-square tombstone at the end.
    assert!(t.contains("#h(1fr)") && t.contains("square"), "no QED tombstone; got:\n{t}");
}

#[test]
fn proof_optional_arg_becomes_title() {
    let t = typ(r"\begin{proof}[Proof of Theorem~\ref{t}]Body.\end{proof}");
    assert!(
        t.contains("*Proof of Theorem") && !t.contains("*Proof.*"),
        "optional proof name not used as title; got:\n{t}"
    );
    assert!(t.contains("square"), "QED still expected; got:\n{t}");
}
