use byetex_core::{convert, ConvertOptions};

fn no_unsupported_warning(src: &str, cmd: &str) -> bool {
    let out = convert(src, &ConvertOptions::default());
    !out.warnings.iter().any(|w| {
        matches!(
            &w.category,
            byetex_core::warnings::Category::UnsupportedCommand { name }
            if name == cmd
        )
    })
}

#[test]
fn usetheme_no_warning() {
    assert!(no_unsupported_warning(r"\usetheme{Madrid}", "\\usetheme"));
}

#[test]
fn usecolortheme_no_warning() {
    assert!(no_unsupported_warning(r"\usecolortheme{beaver}", "\\usecolortheme"));
}

#[test]
fn setbeamertemplate_no_warning() {
    assert!(no_unsupported_warning(
        r"\setbeamertemplate{navigation symbols}{}",
        "\\setbeamertemplate"
    ));
}

#[test]
fn atbeginsection_no_warning() {
    assert!(no_unsupported_warning(r"\AtBeginSection[]{\tableofcontents}", "\\AtBeginSection"));
}

#[test]
fn subtitle_no_warning() {
    assert!(no_unsupported_warning(r"\subtitle{My subtitle}", "\\subtitle"));
}
