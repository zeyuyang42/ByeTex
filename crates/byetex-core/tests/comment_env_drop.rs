//! Regression: `\begin{comment}...\end{comment}` (the `comment` package) is
//! parsed by tree-sitter-latex as a dedicated `comment_environment` node. Its
//! body is a `comment` child (correctly dropped), but the `\begin{comment}` /
//! `\end{comment}` markers leaked verbatim through the default walker and
//! rendered as body text (corpus 2605.22779 ×4). The whole node must be dropped.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn comment_env_drops_body_and_markers() {
    let out = convert("\\begin{comment}\nhidden body\n\\end{comment}\nvisible\n");
    assert!(
        !out.typst.contains("comment"),
        "no `comment` marker text may leak, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("hidden body"),
        "the comment body must be dropped, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("visible"),
        "content after the comment env must survive, got: {}",
        out.typst
    );
}

#[test]
fn comment_env_inline_surrounding_text_preserved() {
    let out = convert("before\n\\begin{comment}x\\end{comment}\nafter\n");
    assert!(
        out.typst.contains("before") && out.typst.contains("after"),
        "surrounding text must survive, got: {}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\begin{comment}") && !out.typst.contains("\\end{comment}"),
        "no raw comment markers may leak, got: {}",
        out.typst
    );
}
