//! Unit 4 — auto-resolution of the `#bibliography(..., style: "...")` argument.
//!
//! The Typst bibliography style is resolved from three signals in priority
//! order: an explicit natbib option (`\usepackage[numbers]{natbib}`), the
//! `\bibliographystyle{X}` bst name, and finally the document-class default
//! (ICML/NeurIPS/ICLR are author-year; IEEEtran/acmart numeric, etc.).
//!
//! Each test puts a real `refs.bib` on disk so `#bibliography("refs.bib", ...)`
//! actually renders, then asserts the emitted style arg.

use std::fs;
use std::path::PathBuf;

use byetex_core::{convert, ConvertOptions};

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("byetex-bibstyle-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const REFS_BIB: &str = "@article{Smith.2024, author={S}, title={T}, year={2024}, journal={J}}\n";

/// Convert a document `preamble` + body (with `\bibliography{refs}` appended)
/// against a real `refs.bib` on disk. `preamble` goes between
/// `\documentclass{...}` (which the caller supplies) and `\begin{document}`.
fn convert_with(name: &str, doc: &str) -> (String, PathBuf) {
    let dir = tmpdir(name);
    fs::write(dir.join("refs.bib"), REFS_BIB).unwrap();
    fs::write(dir.join("paper.tex"), doc).unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    (out.typst, dir)
}

/// Extract the single `#bibliography(...)` line for focused assertions.
fn bib_line(typ: &str) -> String {
    typ.lines()
        .find(|l| l.contains("#bibliography("))
        .unwrap_or_else(|| panic!("no #bibliography(...) line in:\n{typ}"))
        .to_string()
}

#[test]
fn bst_plain_maps_ieee() {
    let (typ, dir) = convert_with(
        "plain",
        "\\documentclass{article}\\begin{document}\nBody.\n\
         \\bibliographystyle{plain}\\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"ieee\""),
        "plain → ieee; got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bst_plainnat_maps_apa() {
    let (typ, dir) = convert_with(
        "plainnat",
        "\\documentclass{article}\\begin{document}\nBody.\n\
         \\bibliographystyle{plainnat}\\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"apa\""),
        "plainnat → apa; got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn natbib_numbers_overrides_authoryear_bst() {
    let (typ, dir) = convert_with(
        "numbers-override",
        "\\documentclass{article}\\usepackage[numbers]{natbib}\\begin{document}\nBody.\n\
         \\bibliographystyle{plainnat}\\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"ieee\""),
        "[numbers] + plainnat → ieee (option wins); got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bst_splncs04_maps_springer_basic() {
    let (typ, dir) = convert_with(
        "splncs04",
        "\\documentclass{article}\\begin{document}\nBody.\n\
         \\bibliographystyle{splncs04}\\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"springer-basic\""),
        "splncs04 → springer-basic; got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn icml_class_default_authoryear() {
    let (typ, dir) = convert_with(
        "icml",
        "\\documentclass{article}\\usepackage{icml2026}\\begin{document}\nBody.\n\
         \\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"apa\""),
        "icml (author-year class, no bst) → apa; got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn neurips_class_default_authoryear() {
    let (typ, dir) = convert_with(
        "neurips",
        "\\documentclass{article}\\usepackage{neurips_2026}\\begin{document}\nBody.\n\
         \\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"apa\""),
        "neurips (author-year class, no bst) → apa; got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ieeetran_class_default_numeric() {
    let (typ, dir) = convert_with(
        "ieeetran",
        "\\documentclass[conference]{IEEEtran}\\begin{document}\nBody.\n\
         \\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"ieee\""),
        "IEEEtran (numeric class default) → ieee; got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn plain_article_no_style_byte_identical() {
    // Plain article, no bst, no natbib → Numeric with no class default →
    // emit NO style arg (today's behavior, must stay byte-identical).
    let (typ, dir) = convert_with(
        "plain-article",
        "\\documentclass{article}\\begin{document}\nBody.\n\
         \\bibliography{refs}\\end{document}\n",
    );
    assert!(
        typ.contains("#bibliography("),
        "must still emit #bibliography(); got:\n{typ}"
    );
    assert!(
        !bib_line(&typ).contains("style:"),
        "plain article must emit NO style arg; got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn natbib_authoryear_overrides_numeric_bst() {
    let (typ, dir) = convert_with(
        "authoryear-override",
        "\\documentclass{article}\\usepackage[authoryear]{natbib}\\begin{document}\nBody.\n\
         \\bibliographystyle{plain}\\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"apa\""),
        "[authoryear] + plain → apa (option wins); got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn acmart_class_default() {
    let (typ, dir) = convert_with(
        "acmart",
        "\\documentclass[sigconf]{acmart}\\begin{document}\nBody.\n\
         \\bibliography{refs}\\end{document}\n",
    );
    assert!(
        bib_line(&typ).contains("style: \"association-for-computing-machinery\""),
        "acmart class default → association-for-computing-machinery; got:\n{}",
        bib_line(&typ)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn natbib_option_forwards_into_input_file() {
    // The `[numbers]` natbib option is in the MAIN preamble, but the
    // `\bibliography{}` lives in an `\input`ed file. `natbib_mode` must
    // propagate parent→child so the included `\bibliography` resolves numeric
    // (`ieee`), not the plainnat author-year default (`apa`). Regression for
    // the forward-propagation gap in `expand_latex_include`.
    let dir = tmpdir("natbib-input");
    fs::write(dir.join("refs.bib"), REFS_BIB).unwrap();
    fs::write(
        dir.join("tail.tex"),
        "\\bibliographystyle{plainnat}\\bibliography{refs}\n",
    )
    .unwrap();
    fs::write(
        dir.join("paper.tex"),
        "\\documentclass{article}\\usepackage[numbers]{natbib}\\begin{document}\n\
         Body.\n\\input{tail}\n\\end{document}\n",
    )
    .unwrap();
    let opts = ConvertOptions {
        source_name: Some("paper.tex".into()),
        base_dir: Some(dir.clone()),
    };
    let out = convert(&fs::read_to_string(dir.join("paper.tex")).unwrap(), &opts);
    assert!(
        bib_line(&out.typst).contains("style: \"ieee\""),
        "[numbers] in the main file must reach an \\input'ed \\bibliography → ieee; got:\n{}",
        bib_line(&out.typst)
    );
    let _ = fs::remove_dir_all(&dir);
}
