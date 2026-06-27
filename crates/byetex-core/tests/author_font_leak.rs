//! Font commands in the author/affiliation block (`\texttt{email}` etc.) leaked
//! their macro name as literal text ("texttt{jane@x.edu") because the affiliation
//! is captured as raw LaTeX and rendered verbatim. Strip the font wrappers,
//! keeping the inner text. Found by the visual grader on 2605.31603 (fairmeta).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn texttt_email_in_author_not_leaked() {
    let t = typ(r"\title{T}\author{Jane Doe\\ \texttt{jane@x.edu}}\begin{document}\maketitle Body.\end{document}");
    assert!(!t.contains("texttt"), "texttt leaked; got:\n{t}");
    assert!(t.contains("jane") && t.contains("x.edu"), "email content lost; got:\n{t}");
}

#[test]
fn textsf_wrapped_email_not_leaked() {
    // The email capture now unwraps ANY \cmd{...} wrapper at the source
    // (extract_email_token via strip_unknown_author_cmds), so wrappers beyond the
    // old downstream fixed list (e.g. \textsf) no longer leak their macro name.
    let t = typ(r"\title{T}\author{Jane Doe\\ \textsf{jane@x.edu}}\begin{document}\maketitle Body.\end{document}");
    assert!(!t.contains("textsf"), "textsf leaked into email; got:\n{t}");
    assert!(t.contains("jane") && t.contains("x.edu"), "email content lost; got:\n{t}");
}
