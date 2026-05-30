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
fn addcontentsline_no_warning() {
    assert!(
        no_unsupported_warning(
            r"\addcontentsline{toc}{section}{Introduction}",
            "\\addcontentsline"
        ),
        "\\addcontentsline should be silently dropped"
    );
}

#[test]
fn addtocontents_no_warning() {
    assert!(
        no_unsupported_warning(
            r"\addtocontents{toc}{\protect\vspace{2ex}}",
            "\\addtocontents"
        ),
        "\\addtocontents should be silently dropped"
    );
}

#[test]
fn surrounding_text_preserved() {
    let src = r"Before.\addcontentsline{toc}{section}{Intro}After.";
    let out = convert(src, &ConvertOptions::default()).typst;
    assert!(
        out.contains("Before") && out.contains("After"),
        "surrounding text must be preserved, got: {out}"
    );
}
