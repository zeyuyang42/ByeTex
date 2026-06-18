//! `\ExplSyntaxOn … \ExplSyntaxOff` brackets expl3 code (`\cs_new:Npn`,
//! `\seq_new:N`, `\tl_set:Nn`, …) whose `:`/`_` catcodes tree-sitter can't parse,
//! so the region used to leak verbatim into the body as garbage text (dogfood
//! backlog F5 — corpus 2605.22821 leaked ~294 such lines). The region is
//! preamble-only and is now skipped wholesale, like `\makeatletter … \makeatother`.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn explsyntax_region_is_skipped() {
    let t = typ(
        "\\documentclass{article}\n\
         \\ExplSyntaxOn\n\
         \\cs_new:Npn \\myfunc #1 { \\tl_set:Nn \\l_tmpa_tl {#1} }\n\
         \\seq_new:N \\l_my_seq\n\
         \\ExplSyntaxOff\n\
         \\begin{document}\nReal body text.\n\\end{document}\n",
    );
    assert!(t.contains("Real body text."), "body must survive; got:\n{t}");
    assert!(!t.contains("cs_new"), "expl3 code must not leak; got:\n{t}");
    assert!(!t.contains("seq_new"), "expl3 code must not leak; got:\n{t}");
    assert!(!t.contains("l_tmpa_tl"), "expl3 vars must not leak; got:\n{t}");
    assert!(!t.contains("Npn"), "expl3 signatures must not leak; got:\n{t}");
}

#[test]
fn explsyntax_does_not_swallow_following_body() {
    // The body immediately after `\ExplSyntaxOff` must be preserved intact.
    let t = typ(
        "\\documentclass{article}\\ExplSyntaxOn\n\\cs_set:Npn \\x {y}\n\\ExplSyntaxOff\n\
         \\begin{document}\nUNIQUEBODY42 here.\n\\end{document}\n",
    );
    assert!(t.contains("UNIQUEBODY42 here."), "body intact; got:\n{t}");
    assert!(!t.contains("cs_set"), "no expl3 leak; got:\n{t}");
}

#[test]
fn non_expl_text_with_underscores_is_unaffected() {
    // Sanity: ordinary body underscores must not be touched by the expl skip.
    let t = typ(
        "\\documentclass{article}\\begin{document}\nsnake_case_word stays.\n\\end{document}\n",
    );
    assert!(t.contains("snake") && t.contains("case"), "ordinary text intact; got:\n{t}");
}
