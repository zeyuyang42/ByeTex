//! Old-style declarative font switches: `{\bf x}`, `{\em y}`, `{\it z}`.
//! Unlike `\textbf{x}` (argument form), these switch the font for the rest of
//! the enclosing group. ByeTex used to warn+drop the command and emit the text
//! unformatted; now it wraps the rest of the group in the matching Typst markup.

use byetex_core::{convert, Category, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

fn warns_unsupported(src: &str, cmd: &str) -> bool {
    convert(src, &ConvertOptions::default())
        .warnings
        .iter()
        .any(|w| matches!(&w.category, Category::UnsupportedCommand { name } if name == cmd))
}

#[test]
fn bf_group_becomes_strong() {
    let t = typ("Normal {\\bf bold words} after.");
    assert!(t.contains("*bold words*"), "got:\n{t}");
    assert!(
        t.contains("Normal") && t.contains("after"),
        "surrounding text must be preserved; got:\n{t}"
    );
}

#[test]
fn em_group_becomes_emph() {
    let t = typ("{\\em emphasized}");
    assert!(t.contains("_emphasized_"), "got:\n{t}");
}

#[test]
fn it_group_becomes_emph() {
    let t = typ("{\\it italic text}");
    assert!(t.contains("_italic text_"), "got:\n{t}");
}

#[test]
fn declarative_switch_no_longer_warns() {
    assert!(
        !warns_unsupported("{\\bf x}", "\\bf"),
        "\\bf must not warn now"
    );
    assert!(
        !warns_unsupported("{\\em y}", "\\em"),
        "\\em must not warn now"
    );
    assert!(
        !warns_unsupported("{\\it z}", "\\it"),
        "\\it must not warn now"
    );
}

#[test]
fn no_stray_brace_left_after_wrap() {
    let t = typ("a {\\bf b} c");
    assert!(!t.contains('}'), "no stray closing brace; got:\n{t}");
    assert!(!t.contains('{'), "no stray opening brace; got:\n{t}");
}
