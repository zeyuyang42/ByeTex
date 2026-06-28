//! authblk's `\affil[n]{body}` (and `\affil{body}`) must not leak its optional
//! `[n]` index or its body into the document. tree-sitter-latex parses the
//! optional `[n]` + `{body}` as siblings of the bare `\affil` generic_command,
//! and the old handler captured the body via `first_curly_like` but never
//! advanced `skip_until`, so both leaked as raw text (dogfood 2605.22728 / and
//! cleanly on 2605.22724, 2605.31394: `\affil[1]{Dept...}` → `\[1\]Dept...`).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
    .typst
}

#[test]
fn affil_with_optional_index_does_not_leak() {
    let src = r"\documentclass{article}\usepackage{authblk}
\author[1]{Alice}
\affil[1]{Department of Mathematics, Example University}
\begin{document}
Body paragraph.
\end{document}";
    let t = typ(src);
    // The escaped optional-index artifact must NOT appear.
    assert!(
        !t.contains(r"\[1\]"),
        "optional `[1]` leaked as `\\[1\\]`:\n{t}"
    );
    // The raw command must NOT appear in the output.
    assert!(!t.contains(r"\affil"), "raw `\\affil` leaked:\n{t}");
    // The affiliation text must NOT appear inside the body paragraph region
    // (it belongs in the author block). The body sentinel is "Body paragraph."
    let body_idx = t.find("Body paragraph.").expect("body present");
    assert!(
        !t[body_idx..].contains("Department of Mathematics"),
        "affiliation body leaked into the document body:\n{t}"
    );
}

#[test]
fn affil_without_optional_arg_does_not_leak() {
    let src = r"\documentclass{article}\usepackage{authblk}
\author{Bob}
\affil{CERN, Geneva}
\begin{document}
Body paragraph.
\end{document}";
    let t = typ(src);
    assert!(!t.contains(r"\affil"), "raw `\\affil` leaked:\n{t}");
    let body_idx = t.find("Body paragraph.").expect("body present");
    assert!(
        !t[body_idx..].contains("CERN, Geneva"),
        "affiliation body leaked into the document body:\n{t}"
    );
}
