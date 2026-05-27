/// Tests for lstlisting / code block environments.
/// These convert verbatim code environments to Typst `#raw(block: true)[...]`.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn lstlisting_emits_raw_block() {
    let src = "\\begin{lstlisting}\nx = 1\n# comment\n\\end{lstlisting}";
    let out = convert(src);
    assert!(
        out.typst.contains("#raw(") && out.typst.contains("block: true"),
        "lstlisting should emit #raw(..., block: true), got: {}",
        out.typst
    );
}

#[test]
fn lstlisting_preserves_content() {
    let src = "\\begin{lstlisting}\nx = 1\n# comment\n\\end{lstlisting}";
    let out = convert(src);
    // Content must appear in the output (possibly \n-escaped in the string literal)
    assert!(
        out.typst.contains("x = 1") || out.typst.contains("x = 1\\n"),
        "lstlisting content must be preserved, got: {}",
        out.typst
    );
}

#[test]
fn lstlisting_hash_not_escaped() {
    // # in Python code must NOT become \# (that's Typst markup escaping).
    // Inside #raw(...) content, # is literal (or \n-escaped).
    let src = "\\begin{lstlisting}\n# comment\n\\end{lstlisting}";
    let out = convert(src);
    assert!(
        !out.typst.contains("\\#"),
        "# inside lstlisting must not be escaped as \\#, got: {}",
        out.typst
    );
}

#[test]
fn lstlisting_no_warnings() {
    let src = "\\begin{lstlisting}\nx = 1\n\\end{lstlisting}";
    let out = convert(src);
    assert!(
        out.warnings.is_empty(),
        "lstlisting should produce no warnings, got: {:?}",
        out.warnings
    );
}

#[test]
fn lstlisting_with_language_option() {
    // [language=Python] or [style=foo] — ignored gracefully, content still preserved
    let src = "\\begin{lstlisting}[language=Python]\nx = 1\n\\end{lstlisting}";
    let out = convert(src);
    assert!(
        out.typst.contains("x = 1") || out.typst.contains("x = 1\\n"),
        "lstlisting with language option must preserve content, got: {}",
        out.typst
    );
    assert!(
        out.warnings.is_empty(),
        "lstlisting with language option should produce no warnings, got: {:?}",
        out.warnings
    );
}
