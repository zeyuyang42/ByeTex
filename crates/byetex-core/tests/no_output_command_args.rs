//! No-output preamble/counter commands (`\setcounter`, `\refstepcounter`,
//! `\setminted`, …) are dropped, but their trailing `[opt]`/`{arg}` groups used to
//! leak into the body when tree-sitter parsed them as siblings — e.g.
//! `\setminted[python]{fontsize=…}` → `\[python\]escapeinside=@@, fontsize=` and
//! `\refstepcounter{ALC@line}` → raw text (dogfood backlog F5, corpus 2605.22821 /
//! 2605.31510). The command now consumes its argument groups too.

use byetex_core::{convert, ConvertOptions};

fn body(inner: &str) -> String {
    convert(
        &format!("\\documentclass{{article}}\\begin{{document}}\n{inner}\nKEEPBODY tail.\n\\end{{document}}"),
        &ConvertOptions::default(),
    )
    .typst
}

#[test]
fn setminted_options_do_not_leak() {
    let t = body(r"\setminted[python]{escapeinside=@@, fontsize=\footnotesize}");
    assert!(!t.contains("python"), "optional [python] must not leak; got:\n{t}");
    assert!(!t.contains("escapeinside"), "options must not leak; got:\n{t}");
    assert!(!t.contains("fontsize"), "options must not leak; got:\n{t}");
    assert!(t.contains("KEEPBODY tail."), "body must survive; got:\n{t}");
}

#[test]
fn setcounter_two_args_do_not_leak() {
    let t = body(r"\setcounter{page}{17}");
    assert!(!t.contains("17"), "counter args must not leak; got:\n{t}");
    assert!(t.contains("KEEPBODY tail."), "body must survive; got:\n{t}");
}

#[test]
fn refstepcounter_arg_does_not_leak() {
    let t = body(r"\refstepcounter{equation}");
    assert!(!t.contains("refstepcounter") && !t.contains("equation"), "must not leak; got:\n{t}");
    assert!(t.contains("KEEPBODY tail."), "body must survive; got:\n{t}");
}

#[test]
fn stepcounter_does_not_leak() {
    let t = body(r"\stepcounter{footnote}");
    assert!(!t.contains("stepcounter") && !t.contains("footnote"), "must not leak; got:\n{t}");
    assert!(t.contains("KEEPBODY tail."), "body must survive; got:\n{t}");
}

#[test]
fn body_brace_group_after_is_not_over_consumed() {
    // A counter command followed by ordinary text (no brace group) must not eat
    // the following sentence.
    let t = body(r"\setcounter{section}{2} Then a normal sentence.");
    assert!(t.contains("Then a normal sentence."), "following text intact; got:\n{t}");
}

// ── regression: over-consumption bugs caught in code review ──────────────────

#[test]
fn no_opt_command_does_not_eat_a_following_bracket() {
    // `\pagestyle` takes no optional arg, so a following `[KEEPME]` is real body
    // content and must survive (only minted-family commands consume `[...]`).
    let t = body(r"\pagestyle{plain} [KEEPME] tail.");
    assert!(t.contains("KEEPME"), "a non-arg [bracket] must not be eaten; got:\n{t}");
}

// NOTE on a pre-existing limitation (NOT this fix): tree-sitter-latex greedily
// attaches a following paragraph's `{...}` group as a second argument of an
// arg-taking command (`\pagestyle{x}\n\n{para}` drops `{para}` on main too), so a
// `{...}`-led paragraph right after one of these commands is lost regardless of our
// handling. That's a grammar issue out of scope here; the consumer's paragraph-stop
// guard only protects the sibling-parsed case.

#[test]
fn setminted_still_consumes_its_optional_arg() {
    // The minted family DOES take `[opt]` — make sure the gate didn't break that.
    let t = body(r"\setminted[python]{fontsize=\small} real body.");
    assert!(!t.contains("python") && !t.contains("fontsize"), "minted opts dropped; got:\n{t}");
    assert!(t.contains("real body."), "body survives; got:\n{t}");
}
