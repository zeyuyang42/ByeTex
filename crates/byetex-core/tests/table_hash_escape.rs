//! Regression tests for Bug #22821 — `escape_text_cell` and `content_escape`
//! must not escape `#` that starts a ByeTex-generated Typst function call.
//!
//! Root causes:
//! 1. `escape_text_cell` escapes ALL `#` to `\#`, breaking `#raw(...)`,
//!    `#link(...)` etc. that ByeTex itself emits in table cells.
//! 2. `content_escape` (in class_map) does the same for the abstract slot,
//!    breaking `#raw(...)` macros that expand in the abstract body.
//! 3. `#raw("BpB")(↓)` — `\xspace` is dropped and Typst greedily parses
//!    the trailing `(↓)` as function arguments; needs a separator.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

/// \texttt{} inside a table cell must not produce `\#raw(...)` —
/// the `#` must survive intact so Typst can call `raw(...)`.
#[test]
fn texttt_in_tabular_cell_no_hash_escape() {
    let src = r"\begin{document}
\begin{tabular}{ll}
  Method & Score \\
  \texttt{BPE} & 0.87 \\
\end{tabular}
\end{document}";
    let t = typst(src);
    assert!(
        !t.contains("\\#raw"),
        "\\#raw found — `#` in cell was escaped; output:\n{t}"
    );
    assert!(
        t.contains("#raw("),
        "#raw( missing from output; output:\n{t}"
    );
}

/// \href{url}{text} inside a table cell must produce `#link(...)`,
/// not `\#link(...)`.
#[test]
fn href_in_tabular_cell_no_hash_escape() {
    let src = r"\begin{document}
\begin{tabular}{ll}
  Resource & Link \\
  Code & \href{https://example.com/code}{here} \\
\end{tabular}
\end{document}";
    let t = typst(src);
    assert!(
        !t.contains("\\#link"),
        "\\#link found — `#` in cell was escaped; output:\n{t}"
    );
    assert!(
        t.contains("#link("),
        "#link( missing from output; output:\n{t}"
    );
}

/// When a \newcommand expands to \texttt{} and is used in a table cell,
/// the generated `#raw(...)` must not have its `#` escaped.
#[test]
fn macro_texttt_in_tabular_cell_no_hash_escape() {
    let src = r"\begin{document}
\newcommand{\mymethod}{\texttt{ConvexTok}}
\begin{tabular}{ll}
  Name & Value \\
  \mymethod & best \\
\end{tabular}
\end{document}";
    let t = typst(src);
    assert!(!t.contains("\\#raw"), "\\#raw found in cell; output:\n{t}");
}

/// \texttt{} expansions in the abstract must not be `\#raw(...)` either.
/// The abstract is placed in `abstract: [...]` via `content_escape`.
/// Uses `\documentclass{article}` so the class is `ArxivArticle` which
/// routes the abstract through the template slot (not the body).
#[test]
fn texttt_in_abstract_no_hash_escape() {
    let src = r"\documentclass{article}
\title{My Paper}
\author{Alice}
\begin{document}
\begin{abstract}
We compare \texttt{BPE} and \texttt{Unigram} tokenisers.
\end{abstract}
Body text.
\end{document}";
    let t = typst(src);
    // In the abstract slot the `#raw(...)` calls must NOT be escaped.
    assert!(
        !t.contains("\\#raw"),
        "\\#raw found in abstract; output:\n{t}"
    );
}

/// `\texttt{X}\xspace(Y)` — after a `#raw("X")` call the trailing `(`
/// must not be parsed by Typst as function arguments.  ByeTex must emit a
/// zero-width separator so the `(Y)` stays in content mode.
///
/// Typst error before the fix: `the character ... is not valid in code`
/// at the first character inside the `(...)`.
#[test]
fn xspace_raw_then_paren_no_code_mode_leak() {
    let src = r"\begin{document}
\newcommand{\bpb}{\texttt{BpB}\xspace}
\begin{tabular}{cc}
  \multicolumn{2}{c}{Validation \bpb(\textdownarrow)} \\
  a & b \\
\end{tabular}
\end{document}";
    let t = typst(src);
    // The cell must not contain `#raw("BpB")(` without a separator
    // (that pattern triggers Typst's function-call chaining on the raw value).
    assert!(
        !t.contains("#raw(\"BpB\")("),
        "#raw(\"BpB\")( found — no separator before `(`; output:\n{t}"
    );
    // The separator must NOT be `#[]` — `#[](↓)` has the same Typst
    // function-call chaining problem as `#raw("BpB")(↓)` because Typst
    // parses the empty content block `[]` as callable and tries to
    // apply `(↓)` as its arguments.
    assert!(
        !t.contains("#raw(\"BpB\")#[]"),
        "#raw(\"BpB\")#[] found — `#[]` separator still chains into `(↓)`; output:\n{t}"
    );
    // A space separator is the correct fix: `#raw("BpB") (↓)`.
    assert!(
        t.contains("#raw(\"BpB\") ("),
        "#raw(\"BpB\") ( (space-separated) not found; output:\n{t}"
    );
}
