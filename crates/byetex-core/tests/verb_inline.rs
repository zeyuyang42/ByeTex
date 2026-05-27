use byetex_core::{convert, ConvertOptions};

fn out(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn no_verb_warning(src: &str) -> bool {
    let result = convert(src, &ConvertOptions::default());
    !result.warnings.iter().any(|w| {
        matches!(
            &w.category,
            byetex_core::warnings::Category::UnsupportedCommand { name }
            if name.starts_with("\\verb")
        )
    })
}

#[test]
fn verb_pipe_delimiter() {
    let o = out(r"\verb|hello world|");
    assert!(o.contains("#raw(\"hello world\")"), "got: {o}");
}

#[test]
fn verb_bang_delimiter() {
    let o = out(r"\verb!some code!");
    assert!(o.contains("#raw(\"some code\")"), "got: {o}");
}

#[test]
fn verb_no_warning() {
    assert!(no_verb_warning(r"\verb|code|"), "\\verb should produce no warning");
}

#[test]
fn verb_star_no_warning() {
    assert!(no_verb_warning(r"\verb*|code|"), "\\verb* should produce no warning");
}

#[test]
fn verb_backslash_escaped() {
    // Backslashes in verb content must be double-escaped for Typst string literal.
    let o = out(r"\verb|\indent|");
    assert!(o.contains("#raw(\"\\\\indent\")"), "got: {o}");
}
