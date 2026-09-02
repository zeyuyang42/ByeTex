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
fn ignores_escaped_bracket_compact_alpha() {
    // A compact `\[..\]` with NO whitespace but an alphabetic token is a literal
    // unit/abbreviation (`[dB]`, `[IU]`, `[mV]`), not a footnote/affiliation marker
    // (those are numeric/symbolic). byetex escapes them correctly; not a leak.
    // (dogfood 2605.31499: `[SNR \[dB\]]` in a table cell false-positived.)
    for typ in ["axis \\[dB\\] label\n", "dose \\[IU\\]\n", "gain \\[mV\\]\n"] {
        let leaks = scan_typ_leaks(typ);
        assert!(leaks.is_empty(), "alphabetic compact literal is not a leak; got {leaks:?} for {typ:?}");
    }
}

#[test]
fn flags_escaped_bracket_footnote_symbol() {
    // A footnote-symbol marker leak (`\[*\]`, `\[†\]`) is still a genuine leak.
    for typ in ["name \\[*\\]\n", "name \\[\u{2020}\\]\n"] {
        let leaks = scan_typ_leaks(typ);
        assert!(!leaks.is_empty(), "footnote-symbol marker is still flagged; got {leaks:?} for {typ:?}");
    }
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

// ── Cosmetic leak vs CONTENT LOSS ───────────────────────────────────────────
//
// Every leak carried the same message ("convert or remove it") and the same
// suggested skill, so an agent could not tell a harmless wrapper token from a
// region where the converter DROPPED mathematics. On 2605.22728 two math regions
// had `\operatorname{div}`, `\tfrac`, `\nabla`, `\alpha`, `\delta` entirely
// absent from the `.typ` — and `byetex-using-warnings-json` says to "translate or
// delete the leaked fragment", which followed literally there yields a
// compiling-but-wrong document. Reported by two dogfood agents.
//
// The scanner only has the `.typ`, so it cannot know the source held more. It
// CAN tell by shape: a leaked `\begin`/`\end`/`\left`/`\right` means a whole
// environment failed to convert and deleting it discards content; a leaked
// `\hspace` is cosmetic.

#[test]
fn a_structural_leak_warns_about_missing_content() {
    let leaks = scan_typ_leaks(r"z in \left\{\begin{aligned} x &= 1 \end{aligned}\right\}");
    assert!(!leaks.is_empty(), "the region is flagged");
    let msg = leaks.iter().map(|d| d.message.as_str()).collect::<Vec<_>>().join(" | ");
    assert!(
        msg.contains("content") || msg.contains("missing") || msg.contains("dropped"),
        "a collapsed environment must warn that content may be MISSING, not just \
         say `convert or remove it`; got: {msg}"
    );
}

#[test]
fn a_structural_leak_routes_to_the_parse_error_skill() {
    // `byetex-parse-error` carries the manual-rewrite guidance that actually
    // applies here; an agent reported it was never reachable because every leak
    // pointed at the generic router.
    let leaks = scan_typ_leaks(r"\begin{aligned} x &= 1 \end{aligned}");
    let skills: Vec<&str> = leaks.iter().filter_map(|d| d.skill_name.as_deref()).collect();
    assert!(
        skills.iter().any(|s| *s == "byetex-parse-error"),
        "a collapsed environment routes to byetex-parse-error; got: {skills:?}"
    );
}

#[test]
fn a_cosmetic_leak_keeps_the_delete_it_advice() {
    // The control: an inline spacing command IS safe to delete, and must not be
    // dressed up as content loss or the warning stops meaning anything.
    let leaks = scan_typ_leaks(r"Some text \hspace{-1mm} more text");
    assert!(!leaks.is_empty(), "still flagged");
    let d = &leaks[0];
    assert!(
        !d.message.contains("missing"),
        "an inline leak is not content loss; got: {}",
        d.message
    );
    assert_eq!(
        d.skill_name.as_deref(),
        Some("byetex-using-warnings-json"),
        "inline leaks keep the generic router"
    );
}

#[test]
fn a_doubled_backslash_command_inside_a_math_string_is_flagged() {
    // `\\hspace{…}` inside a quoted Typst string renders as visible garbage but
    // was invisible: the scanner skips `\\` because `#raw("…")` legitimately
    // doubles backslashes for LaTeX listings. Inside a `$…$` math string it is a
    // real leak. Found only by a manual regex sweep.
    let leaks = scan_typ_leaks(r#"$ x = "a\\hspace{-0.125mm}b" $"#);
    assert!(
        !leaks.is_empty(),
        "a doubled-backslash command in a math string is a leak"
    );
}

#[test]
fn a_doubled_backslash_inside_raw_is_still_ignored() {
    // The control for the rule above: `#raw("…")` really does double backslashes
    // for a LaTeX listing, and those are correctly-rendered code, not leaks.
    let leaks = scan_typ_leaks(r#"#raw("\\textbf{bold}")"#);
    assert!(
        leaks.is_empty(),
        "a LaTeX listing in #raw is not a leak; got: {leaks:?}"
    );
}

// ── Regressions found in review ─────────────────────────────────────────────

#[test]
fn a_doubled_structural_leak_is_still_content_loss() {
    // The doubled-backslash branch bypassed the severity check, so a collapsed
    // environment inside a string — the exact 2605.22728 shape this scan exists
    // for — was told it was safe to delete.
    let leaks = scan_typ_leaks(r#"$ x = "\\begin{aligned} y \\right\}" $"#);
    let msg = leaks.iter().map(|d| d.message.as_str()).collect::<Vec<_>>().join(" | ");
    assert!(
        msg.contains("CONTENT IS LIKELY MISSING"),
        "a doubled structural leak is still content loss; got: {msg}"
    );
    let skills: Vec<&str> = leaks.iter().filter_map(|d| d.skill_name.as_deref()).collect();
    assert!(
        skills.contains(&"byetex-parse-error"),
        "and routes to byetex-parse-error; got: {skills:?}"
    );
}

#[test]
fn both_backslash_forms_on_one_line_are_reported() {
    // The two forms shared a dedup key, so whichever came first swallowed the
    // other — dropping a structural diagnostic entirely.
    let leaks = scan_typ_leaks(r#"$ "\\begin{aligned}" $ and \begin{aligned}"#);
    assert!(
        leaks.len() >= 2,
        "the doubled and single forms are distinct diagnostics; got {} : {leaks:?}",
        leaks.len()
    );
}

#[test]
fn an_inline_argument_command_is_not_called_content_loss() {
    // `\smash`/`\substack` take an argument that survives beside them, so
    // labelling them content loss sends the reader to rebuild a region that only
    // needed its wrapper stripped.
    let leaks = scan_typ_leaks(r"x \smash{y} z");
    let msg = leaks.iter().map(|d| d.message.as_str()).collect::<Vec<_>>().join(" | ");
    assert!(
        !msg.contains("CONTENT IS LIKELY MISSING"),
        "`\\smash` is a wrapper, not a collapsed region; got: {msg}"
    );
}

#[test]
fn a_multi_line_raw_string_does_not_leak_its_body() {
    // `\verb`/`\path`/`\lstinline` escape only `\` and `"`, so a `#raw("…")` can
    // span lines. On a continuation line `#raw(` is absent, and every `\\cmd` in
    // the listing body was reported as a leak.
    let typ = "#raw(\"\\\\textbf{a}\n\\\\textit{b}\")\n";
    let leaks = scan_typ_leaks(typ);
    assert!(
        leaks.is_empty(),
        "a listing body spanning lines is not a leak; got: {leaks:?}"
    );
}
