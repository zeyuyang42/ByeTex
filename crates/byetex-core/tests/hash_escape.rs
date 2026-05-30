//! Regression tests for unescaped # in text content.
//! In Typst, '#' is special (starts function calls). Literal '#' from
//! LaTeX source must be escaped as '\#'. Paper 22817 regression.

fn convert(src: &str) -> byetex_core::ConvertOutput {
    byetex_core::convert(src, &Default::default())
}

/// Returns true if `s` contains a `#` that is not immediately preceded by `\`.
fn has_bare_hash(s: &str) -> bool {
    let mut prev_backslash = false;
    for ch in s.chars() {
        if ch == '#' && !prev_backslash {
            return true;
        }
        prev_backslash = ch == '\\';
    }
    false
}

#[test]
fn latex_escaped_hash_is_preserved() {
    // LaTeX '\#' (already escaped) must stay as '\#' in Typst output,
    // not double-escape to '\\#'.
    let src = r"\# is a wall";
    let out = convert(src);
    assert!(
        !has_bare_hash(&out.typst),
        "bare # must not appear in output, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("\\#"),
        "\\# must appear in output, got: {}",
        out.typst
    );
}

#[test]
fn bare_hash_in_content_environment_is_escaped() {
    // Bare # characters in environment content (e.g. maze grids in tcolorbox)
    // must be escaped so Typst doesn't try to interpret them as function calls.
    let src = "# . # . . D D";
    let out = convert(src);
    assert!(
        !has_bare_hash(&out.typst),
        "bare # must not appear in output, got: {}",
        out.typst
    );
    assert!(
        out.typst.contains("\\#"),
        "# must be escaped as \\# in output, got: {}",
        out.typst
    );
}
