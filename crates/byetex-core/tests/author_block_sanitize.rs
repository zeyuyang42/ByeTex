//! End-to-end: the rendered author block must be CLEAN — no raw LaTeX tokens —
//! and COMPLETE. Drives the full convert() path (the audit-leak fixtures).

use byetex_core::{convert, ConvertOptions};

fn render(class_and_author: &str) -> String {
    let src = format!(
        r"{class_and_author}\title{{T}}\begin{{document}}Body.\end{{document}}"
    );
    convert(&src, &ConvertOptions::default()).typst
}

/// The line(s) of the generated title block that carry author content: between
/// the title text and the abstract/keywords. We assert over the whole output
/// for simplicity since titles/sections here are trivial.
fn assert_clean(typst: &str) {
    for tok in ["\\,", "\\quad", "\\hspace", "\\thanks", "\\textbf", "\\\\", " & ", "\\}"] {
        assert!(
            !typst.contains(tok),
            "author block leaked `{tok}`:\n{typst}"
        );
    }
    // A leading comment percent must never appear at the start of an author line.
    assert!(!typst.contains("[% "), "leaked comment:\n{typst}");
    assert!(!typst.contains("% lead"), "leaked comment text:\n{typst}");
}

#[test]
fn neurips_comma_thinspace_block_is_clean() {
    // Mirrors 2605.22507: leading %, \, separators, trailing \}.
    let typst = render(
        "\\documentclass{article}\\usepackage{neurips_2026}\
         \\author{% lead\nPablo Moreno \\affiliation{UPF} \\email{p@upf.edu} \\and Adrian Müller \\affiliation{ETH}}",
    );
    assert_clean(&typst);
    assert!(typst.contains("Pablo Moreno"), "author 1 missing:\n{typst}");
    assert!(typst.contains("Adrian Müller"), "author 2 missing:\n{typst}");
}

#[test]
fn comma_names_with_shared_lines_split_all_authors() {
    // Mirrors 2605.22776: comma-separated authors, shared \\ affiliation + email.
    let typst = render(
        "\\documentclass{article}\
         \\author{A. Kirpichenko, A. Konstantinov, L. Utkin \\\\ Peter the Great University \\\\ utkin@x.edu}",
    );
    assert_clean(&typst);
    for who in ["Kirpichenko", "Konstantinov", "Utkin"] {
        assert!(typst.contains(who), "missing author {who}:\n{typst}");
    }
    assert!(typst.contains("Peter the Great University"), "shared affiliation missing:\n{typst}");
}

#[test]
fn textbf_quad_group_splits_authors() {
    // Mirrors 2605.22765: \textbf{ A \quad B \quad C } grouped author row.
    let typst = render(
        "\\documentclass{article}\\usepackage{neurips_2026}\
         \\author{\\textbf{ Umut Simsekli \\quad Eric Moulines \\quad Anna Korba }}",
    );
    assert_clean(&typst);
    for who in ["Umut Simsekli", "Eric Moulines", "Anna Korba"] {
        assert!(typst.contains(who), "missing author {who}:\n{typst}");
    }
}

#[test]
fn substantive_thanks_becomes_affiliation_and_email() {
    // Mirrors 2605.22159: article \thanks holds the affiliation + email.
    let typst = render(
        "\\documentclass{article}\
         \\author{Benedikt Grassle\\thanks{Institut fur Mathematik, Universitat Zurich; benedikt@math.uzh.ch}}",
    );
    assert_clean(&typst);
    assert!(typst.contains("Benedikt Grassle"), "name missing/mangled:\n{typst}");
    // The affiliation text from \thanks must appear (not inline-glued to the name).
    assert!(typst.contains("Institut fur Mathematik"), "thanks affiliation missing:\n{typst}");
    assert!(!typst.contains("GrassleInstitut"), "affiliation glued into name:\n{typst}");
}

#[test]
fn equal_contribution_thanks_still_flags_not_affiliation() {
    let typst = render(
        "\\documentclass{article}\\usepackage{neurips_2026}\
         \\author{Alice\\thanks{Equal contribution} \\and Bob}",
    );
    assert_clean(&typst);
    assert!(typst.contains("Alice") && typst.contains("Bob"));
    // "Equal contribution" is a flag, not an affiliation — must not render as text.
    assert!(!typst.contains("Equal contribution"), "equal-contrib text leaked as affiliation:\n{typst}");
}
