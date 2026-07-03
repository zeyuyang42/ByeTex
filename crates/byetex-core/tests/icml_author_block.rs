//! Regression for Bug B (paper 2605.22579): the ICML author-block macros
//! (`\icmlauthor`, `\icmlaffiliation`, `\icmlsetsymbol`,
//! `\icmlcorrespondingauthor`, `\printAffiliationsAndNotice`) are full TeX
//! (\ifcsname / \csname / counters) that byetex's text-substitution macro
//! expander cannot evaluate. When harvested from icml2026.sty and expanded,
//! they leaked raw machinery into the body (`@icmlsymbolequal`,
//! `\@affil\@anon`, `\stepcounter{...}`), and the stray `@icmlsymbolequal`
//! tripped `typst` with `label <icmlsymbolequal> does not exist`.
//!
//! byetex must instead recognise these commands semantically: capture author
//! NAMES, drop the unparseable affiliation/symbol machinery.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

#[test]
fn icml_author_block_does_not_leak_macro_machinery() {
    // Mirrors the real author block. The \icmlauthor/\icmlaffiliation etc.
    // here are NOT pre-harvested (no .sty), which already exercises the
    // dispatch arms; the harvested-macro path is covered by the corpus sweep.
    let src = r"\documentclass{article}
\usepackage{icml2026}
\begin{document}
\twocolumn[
\icmltitle{A Title}
\begin{icmlauthorlist}
\icmlauthor{Meimingwei Li}{equal,yyy}
\icmlauthor{Yuanhao Ding}{equal,sch}
\end{icmlauthorlist}
\icmlaffiliation{yyy}{Department of Statistics, LMU Munich}
\icmlaffiliation{sch}{School of CIE, Henan University}
\icmlcorrespondingauthor{Yuanhao Ding}{yhding@henu.edu.cn}
\icmlkeywords{Machine Learning, ICML}
]
\printAffiliationsAndNotice{\icmlEqualContribution}
Body text here.
\end{document}";
    let out = convert(src);

    // No raw ICML macro machinery may leak into the body.
    for needle in [
        "icmlsymbol",
        "@anon",
        "@affil",
        "stepcounter",
        "\\icmlauthor",
        "\\icmlaffiliation",
        ":=equal",
        "printAffiliationsAndNotice",
    ] {
        assert!(
            !out.typst.contains(needle),
            "leaked ICML machinery {:?} into output:\n{}",
            needle,
            out.typst
        );
    }
    // Author names should be preserved somewhere in the output/metadata.
    assert!(
        out.typst.contains("Meimingwei Li") && out.typst.contains("Yuanhao Ding"),
        "author names were dropped; got:\n{}",
        out.typst
    );
    // Body must survive.
    assert!(
        out.typst.contains("Body text here"),
        "body lost; got:\n{}",
        out.typst
    );
}

#[test]
fn twocolumn_optional_arg_does_not_leak_brackets_or_vskip() {
    // tree-sitter leaves `\twocolumn[ … ]`'s optional top-matter as detached
    // document-level siblings (bare `[`/`]` tokens + a `\vskip 0.3in`). The title
    // block is captured & re-emitted as a spanning float, so the bracket delimiters
    // and the \vskip residue must NOT leak into the body as `\[ … 0.3in … \]`
    // (corpus 2605.22579 / 2606.12411 rendered a stray `[ 0.3in ]` between the
    // abstract and the first section).
    let src = r"\documentclass{article}
\usepackage{icml2026}
\begin{document}
\twocolumn[
\icmltitle{A Title}
\begin{icmlauthorlist}
\icmlauthor{Alice Smith}{a}
\end{icmlauthorlist}
\icmlaffiliation{a}{MIT}
\vskip 0.3in
]
Body text here.
\end{document}";
    let out = convert(src);
    // The leaked `\twocolumn[` / `]` delimiters render as escaped literal brackets.
    assert!(
        !out.typst.contains("\\["),
        "leaked `\\[` bracket delimiter from \\twocolumn[..]; got:\n{}",
        out.typst
    );
    // The `\vskip 0.3in` residue must not leak as bare body text (a legitimate
    // `#v(0.3in)` in the title block is fine — the bare token is the tell).
    assert!(
        !out.typst.contains("\n0.3in") && !out.typst.contains(" 0.3in\n"),
        "leaked bare `0.3in` \\vskip residue into body; got:\n{}",
        out.typst
    );
    // Title, author, and body all still present.
    assert!(out.typst.contains("A Title"), "title lost:\n{}", out.typst);
    assert!(out.typst.contains("Alice Smith"), "author lost:\n{}", out.typst);
    assert!(out.typst.contains("Body text here"), "body lost:\n{}", out.typst);
}
