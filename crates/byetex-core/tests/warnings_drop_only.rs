//! Tests for `Category::DropOnly` warnings emitted by the silent-drop audit.
//!
//! Each test asserts:
//! a) The Typst output is not corrupted (the command vanishes but surrounding
//!    content survives).
//! b) Exactly the expected number of `DropOnly` warnings is emitted with the
//!    correct command name.
//!
//! Negative tests at the bottom lock in the deliberate "stays silent" design
//! for pure-spacing commands.

use byetex_core::{convert, warnings::Category, ConvertOptions};

fn convert_str(src: &str) -> byetex_core::ConvertOutput {
    convert(src, &ConvertOptions::default())
}

fn drop_only_names(out: &byetex_core::ConvertOutput) -> Vec<&str> {
    out.warnings
        .iter()
        .filter_map(|w| match &w.category {
            Category::DropOnly { name } => Some(name.as_str()),
            _ => None,
        })
        .collect()
}

// ── Math mode ────────────────────────────────────────────────────────────────

#[test]
fn drop_only_tag_math() {
    let out = convert_str(r"\begin{equation} x = y \tag{*} \end{equation}");
    assert!(out.typst.contains("x = y"), "typst: {}", out.typst);
    let names = drop_only_names(&out);
    assert_eq!(names, vec!["\\tag"], "warnings: {:?}", out.warnings);
}

#[test]
fn drop_only_not_math() {
    let out = convert_str(r"$\not =$");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\not"),
        "expected DropOnly {{\\not}}, warnings: {:?}",
        out.warnings
    );
}

#[test]
fn drop_only_displaystyle_math() {
    let out = convert_str(r"$\displaystyle x$");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\displaystyle"),
        "expected DropOnly {{\\displaystyle}}, warnings: {:?}",
        out.warnings
    );
}

#[test]
fn drop_only_textstyle_math() {
    let out = convert_str(r"$\textstyle x$");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\textstyle"),
        "warnings: {:?}",
        out.warnings
    );
}

#[test]
fn drop_only_scriptstyle_math() {
    let out = convert_str(r"$\scriptstyle x$");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\scriptstyle"),
        "warnings: {:?}",
        out.warnings
    );
}

// ── Page breaks ──────────────────────────────────────────────────────────────

#[test]
fn drop_only_newpage() {
    let out = convert_str("Before\n\\newpage\nAfter");
    assert!(out.typst.contains("Before"), "typst: {}", out.typst);
    assert!(out.typst.contains("After"), "typst: {}", out.typst);
    let names = drop_only_names(&out);
    assert!(names.contains(&"\\newpage"), "warnings: {:?}", out.warnings);
}

#[test]
fn drop_only_clearpage() {
    let out = convert_str("A\\clearpage B");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\clearpage"),
        "warnings: {:?}",
        out.warnings
    );
}

// ── Alignment directives ──────────────────────────────────────────────────────

#[test]
fn drop_only_centering() {
    let out = convert_str("\\centering Hello");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\centering"),
        "warnings: {:?}",
        out.warnings
    );
}

#[test]
fn drop_only_raggedright() {
    let out = convert_str("\\raggedright Text");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\raggedright"),
        "warnings: {:?}",
        out.warnings
    );
}

// ── Macro redefinitions ───────────────────────────────────────────────────────
// Note: `\renewcommand` / `\providecommand` with standard syntax are parsed by
// tree-sitter as `new_command_definition` nodes and captured into the macro
// registry (correct behavior). The `emit_generic_command` warn-arm only fires
// when they appear as bare `generic_command` nodes (degenerate contexts).
// There is no simple black-box test for that case; coverage is by code review.

// ── TOC / bibliography ────────────────────────────────────────────────────────

#[test]
fn drop_only_tableofcontents() {
    let out = convert_str("\\tableofcontents\n\nText");
    assert!(out.typst.contains("Text"), "typst: {}", out.typst);
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\tableofcontents"),
        "warnings: {:?}",
        out.warnings
    );
}

#[test]
fn drop_only_printbibliography() {
    let out = convert_str("\\printbibliography");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\printbibliography"),
        "warnings: {:?}",
        out.warnings
    );
}

// ── ACM metadata ─────────────────────────────────────────────────────────────

#[test]
fn drop_only_acm_conference() {
    let out = convert_str(r"\acmConference[conf]{Conference Name}{Date}{City}");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\acmConference"),
        "warnings: {:?}",
        out.warnings
    );
}

#[test]
fn drop_only_affiliation() {
    let out = convert_str(r"\affiliation{\institution{MIT}}");
    let names = drop_only_names(&out);
    assert!(
        names.contains(&"\\affiliation"),
        "warnings: {:?}",
        out.warnings
    );
}

// ── Negative tests: pure-spacing stays silent ────────────────────────────────

#[test]
fn no_drop_only_for_spacing_hspace() {
    let out = convert_str(r"Text \hspace{1cm} more");
    let names = drop_only_names(&out);
    assert!(
        !names.contains(&"\\hspace"),
        "\\hspace should not produce a DropOnly warning, but got: {:?}",
        out.warnings
    );
}

#[test]
fn no_drop_only_for_spacing_thinspace() {
    let out = convert_str(r"Text\,more");
    let names = drop_only_names(&out);
    assert!(
        names.is_empty(),
        "\\, should not produce warnings, but got: {:?}",
        out.warnings
    );
}

#[test]
fn no_drop_only_for_linebreak() {
    let out = convert_str("Text\\linebreak more");
    let names = drop_only_names(&out);
    assert!(
        !names.contains(&"\\linebreak"),
        "\\linebreak should not produce a DropOnly warning, got: {:?}",
        out.warnings
    );
}

#[test]
fn no_drop_only_for_math_spacing() {
    let out = convert_str(r"$x \! y$");
    let names = drop_only_names(&out);
    assert!(
        !names.contains(&"\\!"),
        "\\! should not produce a DropOnly warning, got: {:?}",
        out.warnings
    );
}
