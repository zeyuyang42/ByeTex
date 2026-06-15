//! `\usetikzlibrary{...}` is preamble plumbing with no Typst equivalent.
//! tree-sitter-latex parses it as a dedicated `tikz_library_import` node (not a
//! generic command), so it used to fall through the emitter's command handling
//! and leak verbatim into the body. It must be dropped silently, like
//! `\usepackage{tikz}`.

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
fn usetikzlibrary_is_dropped_not_leaked() {
    let src = "\\documentclass{article}\n\
               \\usepackage{tikz}\n\
               \\usetikzlibrary{arrows,positioning}\n\
               \\begin{document}\nBody.\n\\end{document}";
    let out = convert_str(src);
    assert!(
        !out.typst.contains("usetikzlibrary"),
        "\\usetikzlibrary must not leak into the output; got:\n{}",
        out.typst
    );
    assert!(
        !out.typst.contains("arrows,positioning"),
        "the tikz library argument must not leak; got:\n{}",
        out.typst
    );
    // The real body must still be present.
    assert!(
        out.typst.contains("Body."),
        "body content lost; got:\n{}",
        out.typst
    );
}

#[test]
fn usetikzlibrary_emits_no_warning() {
    // Dropping is silent — it's known preamble plumbing, not an unsupported
    // command worth flagging.
    let src = "\\documentclass{article}\n\\usetikzlibrary{calc}\n\
               \\begin{document}\nX\n\\end{document}";
    let out = convert_str(src);
    assert!(
        out.warnings.is_empty(),
        "expected no warnings for \\usetikzlibrary; got: {:?}",
        out.warnings
    );
}
