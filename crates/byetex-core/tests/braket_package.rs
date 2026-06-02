//! Expanded-corpus compile-blocker (2605.31203): a paper with
//! `\usepackage{braket}` uses `\Braket{...}` (19×) and `\braket{...}` (2×).
//! Those macros weren't bundled, so they fell through to the unknown-command
//! path and emitted the quoted macro NAME (`"Braket"`) with the argument
//! DROPPED → garbage math (`unknown variable: raket`) → compile failure.
//!
//! Knuth's `braket` package: `\braket{a|b}` takes ONE argument containing the
//! `|` (unlike the physics package's two-argument `\braket{a}{b}`), so the
//! seed must be gated on `\usepackage{braket}` detection, not shared with the
//! physics table.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(src, &ConvertOptions::default()).typst
}

#[test]
fn braket_package_capitalized_braket_expands() {
    let src = "\\documentclass{article}\n\\usepackage{braket}\n\\begin{document}\n\
        $A = \\Braket{\\hat{H}_{\\mathrm{hf}}}$\n\\end{document}\n";
    let t = typ(src);
    // The macro must EXPAND to angle brackets, not leak its name as text.
    assert!(
        !t.contains("\"Braket\"") && !t.contains("raket"),
        "\\Braket must expand, not emit its name as text; got:\n{t}"
    );
    assert!(
        t.contains("chevron.l") && t.contains("chevron.r"),
        "\\Braket should produce angle brackets (chevron.l/.r); got:\n{t}"
    );
    // The argument must survive (here the accented H).
    assert!(t.contains("hat("), "the \\Braket argument must be kept; got:\n{t}");
}

#[test]
fn braket_package_lowercase_single_arg() {
    // braket-package `\braket{a|b}` is ONE argument; the inner `|` stays.
    let src = "\\documentclass{article}\n\\usepackage{braket}\n\\begin{document}\n\
        $\\braket{a|b}$\n\\end{document}\n";
    let t = typ(src);
    assert!(
        !t.contains("\"braket\""),
        "\\braket must expand, not emit its name as text; got:\n{t}"
    );
    assert!(
        t.contains("chevron.l") && t.contains("chevron.r"),
        "\\braket should produce angle brackets; got:\n{t}"
    );
}

#[test]
fn braket_not_active_without_package() {
    // Regression guard: with no \usepackage{braket}, a 2-arg physics-style
    // \braket (when physics IS loaded) must NOT be shadowed by a 1-arg seed.
    let src = "\\documentclass{article}\n\\usepackage{physics}\n\\begin{document}\n\
        $\\braket{a}{b}$\n\\end{document}\n";
    let t = typ(src);
    // physics \braket{a}{b} -> <a | b>: both args present, with a middle bar.
    assert!(
        t.contains("chevron.l") && t.contains("chevron.r"),
        "physics \\braket must still expand to angle brackets; got:\n{t}"
    );
}
