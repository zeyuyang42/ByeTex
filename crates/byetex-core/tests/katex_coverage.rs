//! KaTeX coverage gap test — updated through Phase 2.
//!
//! Reads `tests/data/katex_extracted.json` (produced by `scripts/extract_katex.py`)
//! and `tests/data/katex_exclusions.toml`, then asserts that every KaTeX command
//! is either:
//!   a) handled by ByeTex's `lookup_math_symbol` or `wrap_for_command_name`, OR
//!   b) listed in `katex_exclusions.toml` (deferred or permanently excluded), OR
//!   c) in the `STRUCTURAL_ARMS` constant below (emitted structurally, not via table), OR
//!   d) seeded as an always-on `KATEX_BUILTIN` macro (Phase 2).
//!
//! To add coverage for a command: implement it in emit.rs or package_macros.rs
//! and remove it from the exclusions file. To defer a command: add it to the
//! exclusions file with a reason and optional phase tag.

use std::collections::HashSet;

/// Names that ByeTex's emit_math_command handles structurally (not via lookup_math_symbol
/// or wrap_for_command_name). These are not flagged as gaps.
const STRUCTURAL_ARMS: &[&str] = &[
    // frac family
    "\\frac",
    "\\tfrac",
    "\\dfrac",
    "\\cfrac",
    "\\sqrt",
    // binom family
    "\\binom",
    "\\dbinom",
    "\\tbinom",
    "\\operatorname",
    "\\operatorname*",
    "\\text",
    "\\mathrm",
    "\\textrm",
    "\\mathnormal",
    // font / accent / enclosure wraps — all handled via wrap_for_command_name;
    // no entries needed here since the coverage test checks that function directly.
    // delimiter sizing
    "\\left",
    "\\right",
    "\\middle",
    // spacing / suppressed
    "\\hspace",
    "\\vspace",
    "\\!",
    "\\linebreak",
    "\\nobreak",
    "\\tag",
    // warn-on-drop arms (emit_math_command issues DropOnly warning then skips)
    "\\not",
    "\\displaystyle",
    "\\textstyle",
    "\\scriptstyle",
    "\\scriptscriptstyle",
];

#[derive(serde::Deserialize)]
struct KatexData {
    symbols: Vec<KatexSymbol>,
    macros: Vec<KatexMacro>,
    functions: Vec<KatexFunction>,
}

#[derive(serde::Deserialize)]
struct KatexSymbol {
    name: String,
}

#[derive(serde::Deserialize)]
struct KatexMacro {
    name: String,
}

#[derive(serde::Deserialize)]
struct KatexFunction {
    names: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Exclusion {
    name: String,
}

#[derive(serde::Deserialize)]
struct Exclusions {
    exclude: Vec<Exclusion>,
}

#[test]
fn katex_coverage_complete() {
    let json: KatexData = serde_json::from_str(include_str!("data/katex_extracted.json"))
        .expect("parse katex_extracted.json");

    let excl_toml: Exclusions = toml::from_str(include_str!("data/katex_exclusions.toml"))
        .expect("parse katex_exclusions.toml");

    let excluded: HashSet<String> = excl_toml.exclude.into_iter().map(|e| e.name).collect();

    let structural: HashSet<&str> = STRUCTURAL_ARMS.iter().copied().collect();

    let mut gaps: Vec<String> = Vec::new();

    for sym in &json.symbols {
        let n = &sym.name;
        if excluded.contains(n) || structural.contains(n.as_str()) {
            continue;
        }
        if byetex_core::__test_support::lookup_math_symbol(n).is_none()
            && byetex_core::__test_support::wrap_for_command_name(n).is_none()
            && !byetex_core::__test_support::is_katex_builtin(n)
        {
            gaps.push(n.clone());
        }
    }

    for mac in &json.macros {
        let n = &mac.name;
        if excluded.contains(n) || structural.contains(n.as_str()) {
            continue;
        }
        if byetex_core::__test_support::lookup_math_symbol(n).is_none()
            && byetex_core::__test_support::wrap_for_command_name(n).is_none()
            && !byetex_core::__test_support::is_katex_builtin(n)
        {
            gaps.push(n.clone());
        }
    }

    for func in &json.functions {
        for n in &func.names {
            if excluded.contains(n) || structural.contains(n.as_str()) {
                continue;
            }
            if byetex_core::__test_support::lookup_math_symbol(n).is_none()
                && byetex_core::__test_support::wrap_for_command_name(n).is_none()
                && !byetex_core::__test_support::is_katex_builtin(n)
            {
                gaps.push(n.clone());
            }
        }
    }

    assert!(
        gaps.is_empty(),
        "KaTeX commands not yet covered by ByeTex \
        (add to katex_exclusions.toml if intentional):\n{}",
        gaps.join("\n")
    );
}
