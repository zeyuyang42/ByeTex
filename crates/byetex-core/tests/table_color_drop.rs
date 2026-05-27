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
fn rowcolor_no_warning() {
    assert!(
        no_unsupported_warning(r"\rowcolor{blue}", "\\rowcolor"),
        "\\rowcolor should be silently dropped"
    );
}

#[test]
fn cellcolor_no_warning() {
    assert!(
        no_unsupported_warning(r"\cellcolor{red!50}", "\\cellcolor"),
        "\\cellcolor should be silently dropped"
    );
}

#[test]
fn arrayrulecolor_no_warning() {
    assert!(
        no_unsupported_warning(r"\arrayrulecolor{gray}", "\\arrayrulecolor"),
        "\\arrayrulecolor should be silently dropped"
    );
}

#[test]
fn columncolor_no_warning() {
    assert!(
        no_unsupported_warning(r"\columncolor{green}", "\\columncolor"),
        "\\columncolor should be silently dropped"
    );
}
