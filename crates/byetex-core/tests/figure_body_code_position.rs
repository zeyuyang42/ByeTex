//! A float body is emitted into CODE position — the first argument of
//! `#figure(...)` — where a bare `#` is a syntax error. The emitter stripped
//! exactly one leading `#`, which is right for the usual single-expression body
//! but not for a float that renders as SEVERAL: `\vspace{-0.3cm}` ahead of a
//! `tabular` produced `v(-0.3cm)#table(…)` and typst refused the whole document
//! with "the character `#` is not valid in code" (corpus 2605.22507).

use byetex_core::{convert, ConvertOptions};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

const TABULAR: &str = r#"\begin{tabular}{cc}
\toprule
A & B \\
\midrule
1 & 2 \\
\bottomrule
\end{tabular}"#;

/// No `#` may sit at the start of an argument line inside a `figure(`/`table(`
/// call — that is the exact shape typst rejects.
fn assert_no_bare_hash_in_code(typst: &str) {
    for (i, line) in typst.lines().enumerate() {
        let t = line.trim_start();
        assert!(
            !t.starts_with(")#") && !t.contains(")#"),
            "line {} puts a `#` in code position: {line}",
            i + 1
        );
    }
}

#[test]
fn vspace_before_a_tabular_does_not_leave_a_hash_in_code_position() {
    // A multi-caption float: the `\vspace` and the `\scalebox`ed tabular land in
    // the SAME rendered run, so the body is two expressions. This is the shape
    // corpus 2605.22507 hits.
    let out = convert(
        &doc(&format!(
            "\\begin{{figure}}[t]\n\\centering\n\\vspace{{-0.3cm}}\n\\scalebox{{0.63}}{{\n{TABULAR}\n}}\n\
             \\caption{{First.}}\n\\label{{tab:a}}\n\
             \\includegraphics[width=0.9\\linewidth]{{x.png}}\n\\caption{{Second.}}\n\\end{{figure}}"
        )),
        &ConvertOptions::default(),
    );
    assert!(
        out.typst.contains("table("),
        "table content dropped:\n{}",
        out.typst
    );
    assert_no_bare_hash_in_code(&out.typst);
    assert!(
        !out.typst.contains("v(-0.3cm)#"),
        "spacing call left a `#` dangling in code position:\n{}",
        out.typst
    );
}

#[test]
fn a_plain_tabular_float_still_emits_the_bare_call_form() {
    // Guard the normal path: a single-expression body keeps `table(...)` bare so
    // the `kind: table` detection (caption placement) still fires.
    let out = convert(
        &doc(&format!(
            "\\begin{{figure}}[t]\n\\centering\n{TABULAR}\n\\caption{{Results.}}\n\\end{{figure}}"
        )),
        &ConvertOptions::default(),
    );
    assert!(
        out.typst.contains("kind: table"),
        "table kind lost:\n{}",
        out.typst
    );
    assert_no_bare_hash_in_code(&out.typst);
}
