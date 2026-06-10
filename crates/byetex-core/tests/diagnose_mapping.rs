//! Unit tests for the pure typst-error → source-fragment mapping
//! (`byetex_core::diagnose::map_typst_errors`). No typst binary needed.

use byetex_core::diagnose::{map_typst_errors, Diagnostic};
use byetex_core::warnings::{Category, Range, Severity, Warning};
use byetex_core::NodeOutput;

fn warning(byte_start: u32, byte_end: u32, skill: &str) -> Warning {
    Warning {
        range: Range {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
            byte_start,
            byte_end,
        },
        category: Category::AmbiguousMath {
            reason: "test".into(),
        },
        severity: Severity::Warning,
        message: "test".into(),
        snippet: "test".into(),
        suggested_skill: Some(skill.into()),
    }
}

#[test]
fn maps_error_to_fragment_and_skill() {
    // The `.typ` line 2 is `#foo`, produced by the LaTeX `\foo` at bytes 0..4.
    let source = "\\foo bar";
    let typst = "ok line\n#foo\nok line 3";
    let source_map = vec![NodeOutput {
        src: (0, 4),
        output: "#foo".to_string(),
    }];
    let warnings = vec![warning(0, 4, "byetex-math")];
    let stderr = "error: unknown variable: foo\n  ┌─ main.typ:2:0\n";

    let diags = map_typst_errors(stderr, typst, source, &source_map, &warnings);
    assert_eq!(diags.len(), 1, "one error parsed; got {diags:?}");
    let d = &diags[0];
    assert_eq!(d.message, "unknown variable: foo");
    assert_eq!(d.line, 2);
    assert_eq!(d.col, 0);
    assert_eq!(d.src_fragment.as_deref(), Some("\\foo"));
    assert_eq!(d.typ_region, "#foo");
    assert_eq!(d.skill_name.as_deref(), Some("byetex-math"));
}

#[test]
fn unmappable_error_yields_null_fragment_and_skill() {
    // The error points at a line whose token matches no node output.
    let source = "\\foo";
    let typst = "ok\n#unrelated\nok";
    let source_map = vec![NodeOutput {
        src: (0, 4),
        output: "#foo".to_string(),
    }];
    let stderr = "error: boom\n  ┌─ main.typ:2:0\n";

    let diags = map_typst_errors(stderr, typst, source, &source_map, &[]);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].src_fragment, None);
    assert_eq!(diags[0].skill_name, None);
}

#[test]
fn empty_stderr_yields_no_diagnostics() {
    let diags: Vec<Diagnostic> = map_typst_errors("", "x", "x", &[], &[]);
    assert!(diags.is_empty());
}
