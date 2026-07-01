//! `scan_typ_leaks` flags un-converted LaTeX that survives into the `.typ` and
//! renders literally (compiles fine, looks wrong). The dogfood loop's #1 repeated
//! wish: `diagnose <.typ>` should surface these, not only compile errors.

use byetex_core::diagnose::scan_typ_leaks;

#[test]
fn flags_leaked_latex_command() {
    let typ = "A paragraph with \\textbf{bold} leaked.\nClean line.\n";
    let leaks = scan_typ_leaks(typ);
    assert_eq!(leaks.len(), 1, "expected one leak; got {leaks:?}");
    assert_eq!(leaks[0].line, 1);
    assert!(leaks[0].message.contains("\\textbf"), "names the command: {}", leaks[0].message);
    assert!(leaks[0].skill_name.is_some(), "carries a repair skill");
}

#[test]
fn flags_escaped_bracket_marker() {
    // The author-block / footnote leak: `\[1\]` renders as literal `[1]`.
    let typ = "Yankai Lin\\[1\\]\n";
    let leaks = scan_typ_leaks(typ);
    assert!(!leaks.is_empty(), "should flag the \\[..\\] marker leak");
    assert_eq!(leaks[0].line, 1);
}

#[test]
fn ignores_escaped_bracket_prose() {
    // byetex escapes a LITERAL `[..]` in prose as `\[..\]`, which Typst renders as
    // `[..]` — correct, NOT a leak. The whitespace-containing span with no math/LaTeX
    // signal is the tell. (corpus 2605.31604: `[text tokens, ...]` false-positived.)
    let typ = "structured as \\[text tokens, representation tokens, pixel patches\\].\n";
    let leaks = scan_typ_leaks(typ);
    assert!(leaks.is_empty(), "prose brackets are a legit escape, not a leak; got {leaks:?}");
}

#[test]
fn flags_escaped_bracket_with_math_signal() {
    // A genuinely leaked display-math block copied verbatim contains math signals
    // (`^`/`_`/`\cmd`) even when it has spaces — still a leak.
    let typ = "energy \\[E = mc^2\\] leaked.\n";
    let leaks = scan_typ_leaks(typ);
    assert!(
        leaks.iter().any(|l| l.message.contains("\\[")),
        "math-bearing \\[..\\] is still flagged; got {leaks:?}"
    );
}

#[test]
fn ignores_clean_typst_and_escapes() {
    // Typst linebreak `\`, single-char escapes (`\#` `\$` `\_` `\&`), and a `#raw`
    // fenced code block with backslashes must NOT be flagged.
    let typ = "Heading \\#1 costs \\$5 and a\\_b plus A&B.\\\n```python\nx = a\\nb\n```\n#strong[real]\n";
    let leaks = scan_typ_leaks(typ);
    assert!(leaks.is_empty(), "no false positives; got {leaks:?}");
}

#[test]
fn escaped_backslash_in_raw_string_not_flagged() {
    // The emitter doubles backslashes inside `#raw("…")` code strings, so a LaTeX
    // listing reads `\\textbf` etc. Those are intentional code, not leaks (code-review).
    let typ = "#raw(lang: \"latex\", \"\\\\textbf{x} \\\\section{y}\")\n";
    let leaks = scan_typ_leaks(typ);
    assert!(leaks.is_empty(), "escaped \\\\cmd in #raw must not be flagged; got {leaks:?}");
}

#[test]
fn single_backslash_command_still_flagged() {
    // A genuine leak — single backslash in ordinary content/math — is still caught.
    let typ = "math: $ \"\\textbf{s.t.}\" $\n";
    let leaks = scan_typ_leaks(typ);
    assert_eq!(leaks.len(), 1, "single-\\ leak still flagged; got {leaks:?}");
}

#[test]
fn dedups_repeated_command_on_one_line() {
    let typ = "\\cite{a} and \\cite{b} and \\cite{c}\n";
    let leaks = scan_typ_leaks(typ);
    assert_eq!(leaks.len(), 1, "one diagnostic per command-name per line; got {leaks:?}");
}
