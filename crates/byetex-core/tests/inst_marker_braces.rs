//! Beamer `\inst{N}` affiliation markers (in `\author` and `\institute`) leaked
//! literal `{`/`}` braces into the title author/institution block. The author
//! name-strip loop handled `\inst` in the name, but the captured `\institute{…}`
//! body keeps its `\inst{1}`/`\inst{2}` markers, and `latex_text_to_typst`
//! (which renders the affiliation) didn't know `\inst` — so the braces leaked
//! (e.g. `…and Author 3 {  Institute…`, `…Country }`). Found by the visual
//! grader re-grade on gh-klb2-beamer.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn inst_markers_do_not_leak_braces_in_author_block() {
    let src = r"\documentclass{beamer}\author{A1\inst{1}, A2\inst{2}}\institute{\inst{1} Univ One\\ \inst{2} Univ Two}\begin{document}\begin{frame}Body.\end{frame}\end{document}";
    let t = typ(src);
    assert!(t.contains("Body."), "lost body; got:\n{t}");
    assert!(t.contains("Univ One") && t.contains("Univ Two"), "lost affiliation text; got:\n{t}");
    // The title/author/institution lines must not contain stray braces from \inst / \institute.
    for line in t.lines().filter(|l| {
        l.contains("author:") || l.contains("institution:") || l.contains("#set document(author")
    }) {
        assert!(
            !line.contains('{') && !line.contains('}'),
            "brace leaked into author/institution line: {line}"
        );
    }
    // The marker command itself must not survive as text.
    assert!(!t.contains(r"\inst"), "\\inst leaked as text; got:\n{t}");
}
