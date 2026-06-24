//! Regression: a user command defined inside `\ExplSyntaxOn…\ExplSyntaxOff`
//! (via `\NewDocumentCommand`) is an expl3 helper whose body is pure expl3
//! code (`\clist_map_inline:nn`, `\seq_gput_right:Nx`, …) producing no document
//! output. The `\ExplSyntaxOn` region itself is skipped, but the macro is still
//! harvested by the prepass, so *calling* it after `\ExplSyntaxOff` expanded
//! its expl3 body into the document text as garbage (dogfood backlog H3,
//! 2605.22821). Such a call must be dropped entirely, arguments included.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const SRC: &str = r"\documentclass{article}\begin{document}
Before.
\ExplSyntaxOn
\NewDocumentCommand{\AppendToList}{m}
 {
  \clist_map_inline:nn { #1 }
   { \seq_gput_right:Nx \g_exceptions_seq { \tl_to_str:n { ##1 } } }
 }
\seq_new:N \g_exceptions_seq
\ExplSyntaxOff
After.
\AppendToList{a,is,of}
End.
\end{document}";

#[test]
fn expl3_helper_call_does_not_leak_body() {
    let t = typ(SRC);
    // The surrounding real text survives.
    assert!(t.contains("Before."), "lost text before region; got:\n{t}");
    assert!(t.contains("After."), "lost text after region; got:\n{t}");
    assert!(t.contains("End."), "lost text after the call; got:\n{t}");
    // The expl3 internals must NOT leak.
    for leak in [
        "clist_map_inline",
        "gput_right",
        "tl_to_str",
        "_seq",
        ":Nx",
        ":nn",
    ] {
        assert!(
            !t.contains(leak),
            "expl3 internal `{leak}` leaked into the body; got:\n{t}"
        );
    }
    // The call's argument text must not leak either.
    assert!(
        !t.contains("a,is,of"),
        "the dropped call's argument leaked; got:\n{t}"
    );
}

#[test]
fn ordinary_macro_with_colon_text_still_expands() {
    // A normal macro whose body contains a colon as punctuation (not an expl3
    // argument signature) must keep expanding normally.
    let src = r"\documentclass{article}\begin{document}
\newcommand{\note}[1]{Note: #1.}
\note{hello}
\end{document}";
    let t = typ(src);
    assert!(t.contains("Note:"), "ordinary colon macro was dropped; got:\n{t}");
    assert!(t.contains("hello"), "ordinary macro arg lost; got:\n{t}");
}
