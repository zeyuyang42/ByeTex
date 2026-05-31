//! `\makeatletter ... \makeatother` wraps low-level TeX (internal macro
//! definitions, `\newcount`, `\csname...\endcsname`, `\def\foo@bar`, counter
//! resets like `\rc@count=1`). tree-sitter-latex can't parse these primitives,
//! so the fragments used to leak into the rendered body (e.g. `=1`, `{}`,
//! `rc@XConst@#1`). The whole region must be skipped — it never produces
//! renderable content. Definitions inside (macros, `\def`, `\let`, theorems,
//! `\newif` flags, tcolorbox/siam envs) are harvested at the skip point so
//! their later body uses keep working — in the main document and in `\input`ed
//! files (whose child emitter has no prepass).

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

// ── #2: definitions inside the region that the prepass does NOT cover ──────────
// \newtheorem / \newtcolorbox / \newsiamthm / \newif register state only during
// the emit walk. The region-skip must still harvest them, or later body uses
// mis-render. (Macros / \def / \let ARE covered by the prepass, so they were
// already safe; these are the gap.)

#[test]
fn newtheorem_defined_in_region_is_registered() {
    let src = "\\documentclass{article}\n\
               \\makeatletter\n\\newtheorem{mythm}{My Theorem}\n\\makeatother\n\
               \\begin{document}\n\\begin{mythm}Statement here.\\end{mythm}\n\\end{document}";
    let out = convert_str(src);
    // A registered theorem renders as a #figure with a supplement; an
    // unregistered one emits an UnsupportedEnvironment warning instead.
    let unsupported = out.warnings.iter().any(|w| {
        matches!(&w.category, byetex_core::Category::UnsupportedEnvironment { name } if name == "mythm")
    });
    assert!(!unsupported, "mythm should be a registered theorem env; warnings: {:?}", out.warnings);
    assert!(
        out.typst.contains("supplement: ["),
        "theorem defined in makeatletter region should render as a theorem block; got:\n{}",
        out.typst
    );
    assert!(out.typst.contains("Statement here."), "theorem body lost; got:\n{}", out.typst);
}

#[test]
fn newif_flag_defined_in_region_is_registered() {
    // Flag defined (default false) inside the region; the conditional is used in
    // the body. With the flag registered, the false branch is dropped (SECRET
    // hidden). Without registration, \ifshowdraft is an unknown command, so
    // SECRET leaks as plain body text.
    let src = "\\documentclass{article}\n\
               \\makeatletter\n\\newif\\ifshowdraft\n\\makeatother\n\
               \\begin{document}\n\\ifshowdraft SECRET\\fi VISIBLE\n\\end{document}";
    let out = convert_str(src);
    assert!(
        !out.typst.contains("SECRET"),
        "false \\newif conditional must hide its branch (flag registered); got:\n{}",
        out.typst
    );
    assert!(out.typst.contains("VISIBLE"), "trailing body lost; got:\n{}", out.typst);
}

#[test]
fn newtcolorbox_defined_in_region_is_transparent() {
    // \newtcolorbox env defined in the region → its \begin/\end must be treated
    // as transparent (body passes through), not UnsupportedEnvironment.
    let src = "\\documentclass{article}\n\
               \\makeatletter\n\\newtcolorbox{mybox}{colback=red}\n\\makeatother\n\
               \\begin{document}\n\\begin{mybox}Boxed content.\\end{mybox}\n\\end{document}";
    let out = convert_str(src);
    let unsupported = out.warnings.iter().any(|w| {
        matches!(&w.category, byetex_core::Category::UnsupportedEnvironment { name } if name == "mybox")
    });
    assert!(!unsupported, "mybox should be registered transparent; warnings: {:?}", out.warnings);
    assert!(out.typst.contains("Boxed content."), "box body lost; got:\n{}", out.typst);
}

// ── #3: the region-closer search must be comment- and word-boundary-aware ──────

#[test]
fn makeatother_in_a_comment_does_not_end_region_early() {
    // A `\makeatother` mentioned in a comment between \makeatletter and the real
    // closer must NOT terminate the skip — otherwise the low-level TeX after the
    // comment (here `\rc@count=1` → stray `=1`) leaks back into the body.
    let src = "\\documentclass{article}\n\
               \\makeatletter\n\
               % restore the at-catcode with \\makeatother once done\n\
               \\rc@count=1\\relax\n\
               \\makeatother\n\
               \\begin{document}\nClean body.\n\\end{document}";
    let out = convert_str(src);
    assert!(
        !out.typst.contains("=1"),
        "a commented \\makeatother ended the region early and leaked internals; got:\n{}",
        out.typst
    );
    assert!(out.typst.contains("Clean body."), "body lost; got:\n{}", out.typst);
}

#[test]
fn makeatother_prefixed_command_is_not_treated_as_closer() {
    // A longer control word sharing the `\makeatother` prefix (e.g.
    // `\makeatotherwise`) must not be matched as the closing token.
    let src = "\\documentclass{article}\n\
               \\makeatletter\n\
               \\makeatotherwise\n\
               \\rc@count=1\\relax\n\
               \\makeatother\n\
               \\begin{document}\nClean body.\n\\end{document}";
    let out = convert_str(src);
    assert!(
        !out.typst.contains("=1"),
        "a \\makeatother-prefixed command was matched as the closer, leaking internals; got:\n{}",
        out.typst
    );
    assert!(out.typst.contains("Clean body."), "body lost; got:\n{}", out.typst);
}
