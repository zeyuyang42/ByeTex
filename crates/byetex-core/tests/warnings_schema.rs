//! Locks the public shape of `warnings.json`. The schema lives at
//! `docs/warnings.schema.json` and is the source of truth for agents reading
//! the sidecar. This test asserts:
//!
//! 1. Every variant of `Category` round-trips through Serde with the
//!    documented `{"kind": "...", ...}` tagging.
//! 2. The keys of `Warning` and `Range` are exactly those listed in the
//!    JSON schema (no fields silently added or removed).

use byetex_core::warnings::{Category, Range, Severity, Warning};

#[test]
fn warning_serializes_with_expected_shape() {
    let w = Warning {
        range: Range {
            start_line: 42,
            start_col: 1,
            end_line: 47,
            end_col: 18,
            byte_start: 1023,
            byte_end: 1184,
        },
        category: Category::Tikz,
        severity: Severity::Warning,
        message: "TikZ picture cannot be auto-converted; see suggested skill.".into(),
        snippet: "\\begin{tikzpicture}...\\end{tikzpicture}".into(),
        suggested_skill: Some("byetex-tikz-to-typst".into()),
    };
    let json = serde_json::to_value(&w).unwrap();

    let obj = json.as_object().expect("warning is an object");
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "category",
            "message",
            "range",
            "severity",
            "snippet",
            "suggested_skill",
        ],
        "Warning keys drifted; update docs/warnings.schema.json AND bump the \
         major version in `Cargo.toml` if this is intentional."
    );

    let range_keys: Vec<&str> = json
        .get("range")
        .and_then(|r| r.as_object())
        .map(|o| {
            let mut ks: Vec<&str> = o.keys().map(String::as_str).collect();
            ks.sort();
            ks
        })
        .unwrap();
    assert_eq!(
        range_keys,
        vec![
            "byte_end",
            "byte_start",
            "end_col",
            "end_line",
            "start_col",
            "start_line",
        ],
    );

    assert_eq!(json["severity"], "warning");
    assert_eq!(json["category"], serde_json::json!({"kind": "tikz"}));
    assert_eq!(json["suggested_skill"], "byetex-tikz-to-typst");
}

#[test]
fn every_category_variant_roundtrips() {
    let cases: Vec<(Category, serde_json::Value)> = vec![
        (
            Category::UnsupportedCommand {
                name: "\\foo".into(),
            },
            serde_json::json!({"kind": "unsupported_command", "name": "\\foo"}),
        ),
        (
            Category::UnsupportedEnvironment {
                name: "tikzpicture".into(),
            },
            serde_json::json!({"kind": "unsupported_environment", "name": "tikzpicture"}),
        ),
        (
            Category::CustomMacro {
                name: "\\mybox".into(),
            },
            serde_json::json!({"kind": "custom_macro", "name": "\\mybox"}),
        ),
        (Category::Tikz, serde_json::json!({"kind": "tikz"})),
        (
            Category::ParseError {
                tree_sitter_node: "ERROR".into(),
            },
            serde_json::json!({"kind": "parse_error", "tree_sitter_node": "ERROR"}),
        ),
        (
            Category::AmbiguousMath {
                reason: "\\foo".into(),
            },
            serde_json::json!({"kind": "ambiguous_math", "reason": "\\foo"}),
        ),
        (
            Category::UnknownPackage {
                name: "tikz".into(),
            },
            serde_json::json!({"kind": "unknown_package", "name": "tikz"}),
        ),
        (
            Category::DropOnly { name: "\\centering".into() },
            serde_json::json!({"kind": "drop_only", "name": "\\centering"}),
        ),
        (
            Category::NeedsManualReview { reason: "x".into() },
            serde_json::json!({"kind": "needs_manual_review", "reason": "x"}),
        ),
    ];

    for (cat, expected) in cases {
        let got = serde_json::to_value(&cat).unwrap();
        assert_eq!(
            got, expected,
            "Category variant serialized differently than the schema documents"
        );
        let back: Category = serde_json::from_value(got).unwrap();
        let again = serde_json::to_value(&back).unwrap();
        assert_eq!(again, expected, "Category round-trip mismatch for {cat:?}");
    }
}
