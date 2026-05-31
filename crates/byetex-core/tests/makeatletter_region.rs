//! `\makeatletter ... \makeatother` wraps low-level TeX (internal macro
//! definitions, `\newcount`, `\csname...\endcsname`, `\def\foo@bar`, counter
//! resets like `\rc@count=1`). tree-sitter-latex can't parse these primitives,
//! so the fragments used to leak into the rendered body (e.g. `=1`, `{}`,
//! `rc@XConst@#1`). The whole region must be skipped — it never produces
//! renderable content. `\newcommand`-family macros defined inside are still
//! harvested by the source prepass, so their later uses keep expanding.

use byetex_core::{convert, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
}

#[test]
fn low_level_tex_in_region_does_not_leak() {
    // Mirrors corpus/2605.22159/source/newcommands.tex.
    let src = "\\documentclass{article}\n\
               \\makeatletter\n\
               \\newcount\\rc@count\n\
               \\rc@count=1\\relax\n\
               \\let\\rc@clearconstantlist\\empty\n\
               \\def\\rc@constname{X}\n\
               \\newcommand\\rc@clearconstant[1]{\\global\\expandafter\\let\\csname rc@XConst@#1\\endcsname\\undefined}\n\
               \\makeatother\n\
               \\begin{document}\nReal body text.\n\\end{document}";
    let out = convert_str(src);
    for leak in ["rc@count", "rc@constname", "rc@XConst", "\\relax", "=1"] {
        assert!(
            !out.typst.contains(leak),
            "makeatletter internal `{leak}` leaked into output; got:\n{}",
            out.typst
        );
    }
    assert!(
        out.typst.contains("Real body text."),
        "body after \\makeatother was lost; got:\n{}",
        out.typst
    );
}

#[test]
fn newcommand_defined_in_region_still_expands() {
    // A `\newcommand` inside the region is harvested by the prepass, so a later
    // use in the body must still expand even though the region is skipped.
    let src = "\\documentclass{article}\n\
               \\makeatletter\n\
               \\newcommand{\\myconst}{EXPANDEDVALUE}\n\
               \\makeatother\n\
               \\begin{document}\nValue: \\myconst.\n\\end{document}";
    let out = convert_str(src);
    assert!(
        out.typst.contains("EXPANDEDVALUE"),
        "macro defined in makeatletter region should still expand in the body; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("\\newcommand") && !out.typst.contains("myconst}"),
        "the definition itself must not leak; got:\n{}",
        out.typst
    );
}

#[test]
fn unmatched_makeatletter_drops_only_the_token() {
    // No `\makeatother` to close: don't swallow the rest of the document —
    // just drop the `\makeatletter` token (and let the body render).
    let src = "\\documentclass{article}\n\\makeatletter\n\
               \\begin{document}\nStill here.\n\\end{document}";
    let out = convert_str(src);
    assert!(
        out.typst.contains("Still here."),
        "unmatched \\makeatletter must not swallow the body; got:\n{}",
        out.typst
    );
}

#[test]
fn makeatletter_region_emits_no_leak_warnings_for_body() {
    // The body following the region converts cleanly.
    let src = "\\documentclass{article}\n\
               \\makeatletter\n\\def\\x@y{1}\n\\makeatother\n\
               \\begin{document}\n\\section{Intro}\nText.\n\\end{document}";
    let out = convert_str(src);
    assert!(out.typst.contains("= Intro"), "heading lost; got:\n{}", out.typst);
    assert!(!out.typst.contains("x@y"), "def leaked; got:\n{}", out.typst);
}
