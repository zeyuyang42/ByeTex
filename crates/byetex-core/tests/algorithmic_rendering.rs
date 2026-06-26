//! `\begin{algorithmic}` pseudocode (algorithmicx/algpseudocode) was collapsed
//! into a single `align(left)[…]` prose line with every `\State`/`\For`/`\Require`
//! keyword DROPPED. Render it as structured lines: one per statement, with bold
//! control keywords (for/if/while/return/Require:), indentation by nesting depth,
//! and line numbers; the `algorithm` float carries kind `algorithm` so its caption
//! reads "Algorithm N". Found by the visual grader on 2605.22549 (4 algorithms,
//! all structure lost).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const ALG: &str = r"\begin{algorithm}\caption{Demo}\begin{algorithmic}[1]
\Require $x>0$
\State $y \gets x$
\For{$i=1$ to $n$}
\State $y \gets y+i$
\EndFor
\Return $y$
\end{algorithmic}\end{algorithm}";

#[test]
fn keywords_are_bold() {
    let t = typ(ALG);
    for kw in ["Require:", "for", "end for", "return"] {
        assert!(
            t.contains(&format!("strong[{kw}]")),
            "keyword `{kw}` not bold; got:\n{t}"
        );
    }
}

#[test]
fn float_kind_is_algorithm() {
    let t = typ(ALG);
    assert!(
        t.contains("kind: \"algorithm\""),
        "algorithm float should set kind: algorithm (→ 'Algorithm N' caption); got:\n{t}"
    );
}

#[test]
fn nested_state_is_indented() {
    let t = typ(ALG);
    // The `\State` inside the `\For` body is indented one level (`#h(...)`),
    // while the top-level `\Require` line is not.
    assert!(
        t.contains("#h("),
        "nested pseudocode lines should be indented; got:\n{t}"
    );
}

#[test]
fn statements_not_collapsed_to_one_line() {
    let t = typ(ALG);
    // The old behavior glued every formula into one align(left)[…] with no breaks.
    assert!(
        !t.contains("align(left)[$x>0$ $y arrow.l x$"),
        "pseudocode statements are still collapsed; got:\n{t}"
    );
}
