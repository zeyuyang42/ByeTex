//! Some classes define `\abstract` as a COMMAND (`\renewcommand{\abstract}[1]{…}`,
//! e.g. bytedance_seed.cls in corpus 2605.31604) rather than the `abstract`
//! environment. ByeTex dropped the bare `\abstract{…}` command ("raw source
//! dropped"), losing the entire abstract — a chunk of the missing content (that
//! paper's word_recall was 0.673). The command form is now captured like the env.
//! (The abstract only renders inside the title block, so each case has `\title` +
//! `\maketitle`.)

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn abstract_command_content_is_kept() {
    let t = typ(
        "\\documentclass{article}\\title{T}\\abstract{This is the ABSTRACTBODY text.}\\begin{document}\\maketitle\nBody.\n\\end{document}",
    );
    assert!(
        t.contains("This is the ABSTRACTBODY text."),
        "the \\abstract{{}} command content must survive; got:\n{t}"
    );
    assert!(
        !t.contains("\\abstract"),
        "raw \\abstract command must not leak; got:\n{t}"
    );
}

#[test]
fn body_abstract_group_is_not_hoisted_into_the_abstract() {
    // Guard (code-review finding): a `\abstract` in the BODY followed by an
    // incidental `{...}` group must NOT move that group into the title-block
    // abstract — only the preamble command form is captured.
    let t = typ(
        "\\documentclass{article}\\title{T}\\begin{document}\\maketitle\\abstract{GROUPSTAYSBODY content}\nrest\\end{document}",
    );
    // The group must not be pulled into the centered Abstract block. (It may render
    // inline in the body or be dropped, but it must not be hoisted to the abstract.)
    let before_body = t.split("GROUPSTAYSBODY").next().unwrap_or("");
    assert!(
        !before_body.contains("Abstract"),
        "a body \\abstract group must not be hoisted into the abstract; got:\n{t}"
    );
}

#[test]
fn preamble_abstract_renders_without_maketitle() {
    // Finding 2 check: a preamble `\abstract{}` with no `\maketitle` still renders
    // (flushed in finish()), not silently lost.
    let t = typ(
        "\\documentclass{article}\\title{T}\\abstract{NOMAKETITLE abstract.}\\begin{document}\nBody.\\end{document}",
    );
    assert!(t.contains("NOMAKETITLE abstract."), "abstract flushed in finish(); got:\n{t}");
}

#[test]
fn abstract_environment_still_wins() {
    // The environment form must keep working and not be clobbered.
    let t = typ(
        "\\documentclass{article}\\title{T}\\begin{document}\\maketitle\\begin{abstract}ENV abstract body.\\end{abstract}\nBody.\\end{document}",
    );
    assert!(t.contains("ENV abstract body."), "abstract env still captured; got:\n{t}");
}
