//! Task 1: every document — regardless of `\documentclass` — is rendered with
//! a self-generated, self-contained "clean neutral article" preamble. No Typst
//! Universe package is imported; the output compiles on stock Typst with no
//! `typst.toml`. Title/authors/abstract are kept via the generated title block.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

const IEEE: &str = "\\documentclass{IEEEtran}\n\\title{My Title}\n\\author{Alice}\n\\begin{document}\n\\begin{abstract}An abstract.\\end{abstract}\n\\section{Intro}\nBody text.\n\\end{document}";
const ACM: &str = "\\documentclass[sigconf]{acmart}\n\\title{My Title}\n\\author{Alice}\n\\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";
const ARTICLE: &str = "\\documentclass{article}\n\\title{My Title}\n\\author{Alice}\n\\begin{document}\n\\section{Intro}\nBody.\n\\end{document}";

#[test]
fn templated_classes_emit_no_universe_import() {
    for (name, src) in [("IEEE", IEEE), ("ACM", ACM)] {
        let t = typ(src);
        assert!(
            !t.contains("@preview"),
            "{name}: must not import a Typst Universe package; got:\n{t}"
        );
        assert!(
            !t.contains("#show: "),
            "{name}: must not use a package show-rule; got:\n{t}"
        );
    }
}

#[test]
fn every_class_gets_the_neutral_preamble() {
    for (name, src) in [("IEEE", IEEE), ("ACM", ACM), ("article", ARTICLE)] {
        let t = typ(src);
        assert!(t.contains("#set page("), "{name}: neutral page setup missing; got:\n{t}");
        assert!(t.contains("#set text("), "{name}: neutral text setup missing; got:\n{t}");
        assert!(
            t.contains("#set par("),
            "{name}: neutral paragraph setup missing; got:\n{t}"
        );
    }
}

#[test]
fn title_block_preserved_for_templated_class() {
    // The generated title block must still carry the title (content preserved).
    let t = typ(IEEE);
    assert!(t.contains("#align(center)"), "title block missing; got:\n{t}");
    assert!(t.contains("My Title"), "title text lost; got:\n{t}");
    assert!(t.contains("An abstract."), "abstract lost; got:\n{t}");
}
