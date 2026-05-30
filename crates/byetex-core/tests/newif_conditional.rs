//! `\newif\ifX` defines a boolean flag (default false) plus the setters
//! `\Xtrue` / `\Xfalse`. ByeTex tracks the flag state and evaluates
//! `\ifX ... \else ... \fi` by emitting only the taken branch — none of the
//! `\newif` machinery should leak into the output or warn.
//!
//! Tree-sitter gives `\newif` no dedicated node (bare `generic_command`s) and
//! does not structure `\if...\else...\fi`, so the branch bounds are found by a
//! depth-aware source scan.

use byetex_core::{convert, Category, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn unsupported_names(src: &str) -> Vec<String> {
    convert(src, &ConvertOptions::default())
        .warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::UnsupportedCommand { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn default_false_drops_the_then_branch() {
    let t = typ("\\newif\\iffoo\n\\iffoo HIDDENBODY\\fi AFTERTEXT");
    assert!(
        !t.contains("HIDDENBODY"),
        "false flag → body dropped; got:\n{t}"
    );
    assert!(
        t.contains("AFTERTEXT"),
        "text after \\fi must remain; got:\n{t}"
    );
}

#[test]
fn true_flag_keeps_the_then_branch() {
    let t = typ("\\newif\\iffoo\n\\footrue\n\\iffoo SHOWNBODY\\fi");
    assert!(t.contains("SHOWNBODY"), "true flag → body kept; got:\n{t}");
}

#[test]
fn false_flag_takes_else_branch() {
    let t = typ("\\newif\\iffoo\n\\iffoo THENPART\\else ELSEPART\\fi");
    assert!(
        t.contains("ELSEPART"),
        "false → else branch kept; got:\n{t}"
    );
    assert!(
        !t.contains("THENPART"),
        "false → then branch dropped; got:\n{t}"
    );
}

#[test]
fn true_flag_takes_then_branch() {
    let t = typ("\\newif\\iffoo\n\\footrue\n\\iffoo THENPART\\else ELSEPART\\fi");
    assert!(t.contains("THENPART"), "true → then branch kept; got:\n{t}");
    assert!(
        !t.contains("ELSEPART"),
        "true → else branch dropped; got:\n{t}"
    );
}

#[test]
fn false_after_true_toggles_state() {
    let t = typ("\\newif\\iffoo\n\\footrue\\foofalse\n\\iffoo NOPART\\fi YESPART");
    assert!(
        !t.contains("NOPART"),
        "last setter (false) wins → body dropped; got:\n{t}"
    );
    assert!(t.contains("YESPART"), "trailing text remains; got:\n{t}");
}

#[test]
fn nested_same_flag_respects_depth() {
    // Outer flag false → the entire outer conditional (including the inner
    // \fi) is dropped; only text after the OUTER \fi survives.
    let t = typ("\\newif\\ifa\n\\ifa XOUT\\ifa YIN\\fi ZOUT\\fi TAILTEXT");
    assert!(
        !t.contains("XOUT") && !t.contains("YIN") && !t.contains("ZOUT"),
        "nested false conditional fully dropped; got:\n{t}"
    );
    assert!(
        t.contains("TAILTEXT"),
        "text after the OUTER \\fi remains; got:\n{t}"
    );
}

#[test]
fn newif_machinery_does_not_warn() {
    let names = unsupported_names("\\newif\\iffoo\n\\footrue\n\\iffoo A\\else B\\fi");
    let bad: Vec<_> = names
        .iter()
        .filter(|n| {
            n.contains("newif")
                || n.contains("iffoo")
                || n.contains("footrue")
                || n.contains("foofalse")
                || n.as_str() == "\\else"
                || n.as_str() == "\\fi"
        })
        .collect();
    assert!(
        bad.is_empty(),
        "\\newif machinery must not warn; got: {bad:?}"
    );
}
