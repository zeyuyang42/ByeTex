//! `\algnewcommand` / `\algrenewcommand` (algorithmicx package) define macros
//! with exactly the same syntax as `\newcommand` / `\renewcommand`, but
//! tree-sitter-latex has no built-in keyword for them, so they parse as bare
//! `generic_command` nodes (like `\newcommandx`). Before the fix the definition
//! body leaked into the document as raw text (dogfood 2605.31499:
//! `\algnewcommand{\LeftComment}[1]{\Statex \(\triangleright\) #1}` →
//! `\[1\] $gt.tri$ \#1`) and the macro was never registered.

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
fn algnewcommand_braced_name_with_arg_does_not_leak() {
    // `\algnewcommand{\name}[1]{body #1}` — the braced-name + arity form.
    let src = r"\documentclass{article}\usepackage{algpseudocode}
\algnewcommand{\LeftComment}[1]{\textbf{Note:} #1}
\begin{document}
Text \LeftComment{hello} more.
\end{document}";
    let t = typ(src);
    // The definition body must NOT leak as raw text.
    assert!(
        !t.contains(r"\[1\]") && !t.contains(r"\#1") && !t.contains("#1"),
        "definition body leaked into output:\n{t}"
    );
    // The macro should expand at the call site (the arg text survives).
    assert!(
        t.contains("hello"),
        "expected the \\LeftComment{{hello}} call to expand and keep `hello`:\n{t}"
    );
}

#[test]
fn algnewcommand_bare_name_does_not_leak() {
    // `\algnewcommand\name{body}` — the bare-name form (no braces, no arity).
    let src = r"\documentclass{article}\usepackage{algpseudocode}
\algnewcommand\algorithmicinput{\textbf{Input:}}
\begin{document}
Before \algorithmicinput{} after.
\end{document}";
    let t = typ(src);
    assert!(
        !t.contains(r"\algorithmicinput") && !t.contains("algnewcommand"),
        "bare-name definition leaked:\n{t}"
    );
    // The body should expand at the call site.
    assert!(t.contains("Input:"), "expected expanded body `Input:`:\n{t}");
}

#[test]
fn algrenewcommand_does_not_leak() {
    let src = r"\documentclass{article}\usepackage{algpseudocode}
\algrenewcommand\algorithmicwhile{\textbf{while}}
\begin{document}
Loop \algorithmicwhile{} body.
\end{document}";
    let t = typ(src);
    assert!(
        !t.contains(r"\algorithmicwhile") && !t.contains("algrenewcommand"),
        "algrenewcommand definition leaked:\n{t}"
    );
    assert!(t.contains("while"), "expected expanded body `while`:\n{t}");
}
