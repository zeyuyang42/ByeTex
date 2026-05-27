use byetex_core::{convert, ConvertOptions};

fn out(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn no_unsupported_warning(src: &str, cmd: &str) -> bool {
    let result = convert(src, &ConvertOptions::default());
    !result.warnings.iter().any(|w| {
        matches!(
            &w.category,
            byetex_core::warnings::Category::UnsupportedCommand { name }
            if name == cmd
        )
    })
}

#[test]
fn hologo_latex() {
    let o = out(r"\hologo{LaTeX}");
    assert!(o.contains("LaTeX"), "got: {o}");
}

#[test]
fn hologo_tex() {
    let o = out(r"\hologo{TeX}");
    assert!(o.contains("TeX"), "got: {o}");
}

#[test]
fn hologo_bibtex() {
    let o = out(r"\hologo{BibTeX}");
    assert!(o.contains("BibTeX"), "got: {o}");
}

#[test]
fn hologo_xelatex() {
    let o = out(r"\hologo{XeLaTeX}");
    assert!(o.contains("XeLaTeX"), "got: {o}");
}

#[test]
fn hologo_no_warning() {
    assert!(
        no_unsupported_warning(r"\hologo{LaTeX}", "\\hologo"),
        "\\hologo should not warn"
    );
}

#[test]
fn hologo_uppercase_variant_no_warning() {
    assert!(
        no_unsupported_warning(r"\Hologo{LaTeX}", "\\Hologo"),
        "\\Hologo should not produce an UnsupportedCommand warning"
    );
}
