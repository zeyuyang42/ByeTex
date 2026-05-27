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
fn doublespacing_no_warning() {
    assert!(
        no_unsupported_warning(r"\doublespacing", "\\doublespacing"),
        "\\doublespacing should be silently dropped"
    );
}

#[test]
fn singlespacing_no_warning() {
    assert!(
        no_unsupported_warning(r"\singlespacing", "\\singlespacing"),
        "\\singlespacing should be silently dropped"
    );
}

#[test]
fn onehalfspacing_no_warning() {
    assert!(
        no_unsupported_warning(r"\onehalfspacing", "\\onehalfspacing"),
        "\\onehalfspacing should be silently dropped"
    );
}

#[test]
fn setstretch_no_warning() {
    assert!(
        no_unsupported_warning(r"\setstretch{1.5}", "\\setstretch"),
        "\\setstretch should be silently dropped"
    );
}
