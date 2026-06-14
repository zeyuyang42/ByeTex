//! `\newtcbtheorem[init]{name}{title}{options}{prefix}` (tcolorbox theorem
//! definition) was unhandled — its multi-group args (esp. the big `{options}`
//! with nested braces) leaked into the body and cascaded into the following
//! preamble content (the NeurIPS `\author{}` block), whose `\\$^3$` then leaked a
//! stray `\$` that broke `$`-math pairing → 98 cascading errors (corpus
//! 2605.31063). Fix: consume the command and all its arguments.

use byetex_core::{convert, ConvertOptions};

fn typst(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn newtcbtheorem_args_do_not_leak() {
    let src = "\\newtcbtheorem[number within=section]{boxedprop}{Proposition}{\n\
        colback=green!3,\n\
        boxed title style={size=small, colframe=green!75},\n\
        before skip=5pt,\n\
        }{prop}\n\
        Body paragraph.";
    let t = typst(src);
    assert!(
        !t.contains("colback") && !t.contains("before skip") && !t.contains("boxed title"),
        "the \\newtcbtheorem options must not leak into the body;\noutput:\n{t}"
    );
    assert!(t.contains("Body paragraph."), "real body must survive;\noutput:\n{t}");
}

#[test]
fn newtcbtheorem_does_not_cascade_into_following_author() {
    // The leak-cascade reproduction: a complex \newtcbtheorem immediately
    // followed by an author block must leave the author block clean.
    let src = "\\newtcbtheorem[number within=section]{boxedprop}{Proposition}{\n\
        colback=green!3, fonttitle=\\bfseries,\n\
        boxed title style={size=small, boxrule=0pt, colframe=green!75},\n\
        before skip=5pt, after skip=5pt,\n\
        }{prop}\n\
        \\title{T}\n\
        \\author{Alice$^{1}$ \\and Bob$^{2}$ \\\\$^1$Univ A}\n\
        \\begin{document}\\maketitle Body.\\end{document}";
    let t = typst(src);
    assert!(
        !t.contains("begin{tabular}") && !t.contains("Univ A"),
        "the author block must not leak into the body;\noutput:\n{t}"
    );
}
