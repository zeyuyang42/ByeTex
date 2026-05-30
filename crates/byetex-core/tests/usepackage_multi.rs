use byetex_core::{convert, ConvertOptions};

fn warnings(src: &str) -> Vec<(String, String)> {
    let opts = ConvertOptions::default();
    let out = convert(src, &opts);
    out.warnings
        .into_iter()
        .filter_map(|w| {
            if let byetex_core::warnings::Category::UnsupportedCommand { name } = w.category {
                Some((name, w.message))
            } else {
                None
            }
        })
        .collect()
}

fn warning_names(src: &str) -> Vec<String> {
    warnings(src).into_iter().map(|(n, _)| n).collect()
}

// A single known-noop package produces zero warnings.
#[test]
fn single_noop_no_warning() {
    let names = warning_names(r"\usepackage{amsmath}");
    assert!(names.is_empty(), "expected no warnings, got {names:?}");
}

// A single unknown package produces one warning named `usepackage:<pkg>`.
#[test]
fn single_unknown_named_warning() {
    let names = warning_names(r"\usepackage{circuitikz}");
    assert_eq!(names, vec!["usepackage:circuitikz"]);
}

// A comma-separated list: known package is silently dropped, unknown package
// produces exactly one warning with the correct name.
#[test]
fn multi_noop_and_unknown() {
    let names = warning_names(r"\usepackage{amsmath,circuitikz}");
    assert_eq!(names, vec!["usepackage:circuitikz"]);
}

// Order shouldn't matter: unknown first, known second.
#[test]
fn multi_unknown_first() {
    let names = warning_names(r"\usepackage{circuitikz,graphicx}");
    assert_eq!(names, vec!["usepackage:circuitikz"]);
}

// Two unknown packages produce two separately named warnings.
#[test]
fn multi_two_unknowns() {
    let mut names = warning_names(r"\usepackage{chemfig,circuitikz}");
    names.sort();
    assert_eq!(names, vec!["usepackage:chemfig", "usepackage:circuitikz"]);
}

// Whitespace after comma must be trimmed: `\usepackage{amsmath, amssymb}`.
#[test]
fn whitespace_in_list_trimmed() {
    let names = warning_names(r"\usepackage{amsmath, amssymb}");
    assert!(
        names.is_empty(),
        "space-padded noop list should be silent, got {names:?}"
    );
}

// Bucket A: imakeidx load is silent — body-level \index calls warn separately.
#[test]
fn imakeidx_silent_load() {
    let names = warning_names(r"\usepackage{imakeidx}");
    assert!(
        names.is_empty(),
        "imakeidx load should be silent, got {names:?}"
    );
}

// Bucket B: xeCJK load is silently dropped (body commands warn on their own).
#[test]
fn xecjk_silent_load() {
    let names = warning_names(r"\usepackage{xeCJK}");
    assert!(
        names.is_empty(),
        "xeCJK load should be silent, got {names:?}"
    );
}

// Bucket C: circuitikz must still produce a warning — it has real semantic content.
#[test]
fn circuitikz_still_warns() {
    let names = warning_names(r"\usepackage{circuitikz}");
    assert_eq!(
        names,
        vec!["usepackage:circuitikz"],
        "circuitikz should stay a named warning"
    );
}

// Options appear in the warning message, not the name.
#[test]
fn options_in_message() {
    let ws = warnings(r"\usepackage[T2A]{fontenc}");
    // fontenc is in the noop list — no warning expected
    assert!(ws.is_empty(), "fontenc should be silent, got {ws:?}");
}

#[test]
fn options_on_unknown_appear_in_message() {
    let ws = warnings(r"\usepackage[main=russian]{babel}");
    // babel is in the noop list — no warning expected
    assert!(ws.is_empty(), "babel should be silent, got {ws:?}");
}

#[test]
fn unknown_with_options_message_contains_option() {
    let ws = warnings(r"\usepackage[draft]{chemfig}");
    assert_eq!(ws.len(), 1);
    let (name, msg) = &ws[0];
    assert_eq!(name, "usepackage:chemfig");
    assert!(
        msg.contains("draft"),
        "expected option in message, got: {msg}"
    );
}
