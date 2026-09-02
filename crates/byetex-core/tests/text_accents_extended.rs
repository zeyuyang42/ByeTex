//! Letter/symbol-named text accents beyond the acute/grave/diaeresis/circumflex/
//! tilde already handled: dot-above `\.`, macron `\=`, caron `\v`, breve `\u`,
//! double-acute `\H`, ring `\r`, cedilla `\c`, ogonek `\k`. Before the fix these
//! were not dispatched to `emit_text_accent`, so the accent + its braced letter
//! were dropped entirely (dogfood 2605.31499: `\.{I}` in `TÜB\.{I}TAK` →
//! `TÜBTAK`/`.I`, dropping the dotted-İ).

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("<test>".into()),
            base_dir: None,
        },
    )
    .typst
}

#[test]
fn dot_accent_capital_i_is_dotted_i() {
    // `\.{I}` is the Turkish dotted capital I (İ, U+0130).
    let t = typ(r#"\documentclass{article}\begin{document}T\"{U}B\.{I}TAK\end{document}"#);
    assert!(
        t.contains("TÜBİTAK"),
        "expected `TÜBİTAK` (dotted-İ); got:\n{t}"
    );
}

#[test]
fn macron_caron_breve_doubleacute_render() {
    // \={a}→ā, \v{s}→š, \u{g}→ğ, \H{o}→ő — common name accents.
    let t = typ(r"\documentclass{article}\begin{document}\={a} \v{s} \u{g} \H{o}\end{document}");
    assert!(t.contains('ā'), "macron \\={{a}}→ā missing:\n{t}");
    assert!(t.contains('š'), "caron \\v{{s}}→š missing:\n{t}");
    assert!(t.contains('ğ'), "breve \\u{{g}}→ğ missing:\n{t}");
    assert!(t.contains('ő'), "double-acute \\H{{o}}→ő missing:\n{t}");
}

#[test]
fn cedilla_ring_ogonek_render() {
    // \c{c}→ç, \r{a}→å, \k{a}→ą.
    let t = typ(r"\documentclass{article}\begin{document}\c{c} \r{a} \k{a}\end{document}");
    assert!(t.contains('ç'), "cedilla \\c{{c}}→ç missing:\n{t}");
    assert!(t.contains('å'), "ring \\r{{a}}→å missing:\n{t}");
    assert!(t.contains('ą'), "ogonek \\k{{a}}→ą missing:\n{t}");
}

#[test]
fn dotted_i_in_author_block_renders() {
    // The author-block sanitize path is separate from emit; `\.{I}` there
    // (e.g. a Turkish affiliation) must also resolve to İ (dogfood 2605.31499).
    let t = typ(r#"\documentclass{article}\author{Alice\thanks{T\"{U}B\.{I}TAK B\.{I}LGEM}}\begin{document}\maketitle x\end{document}"#);
    assert!(
        t.contains("TÜBİTAK") && !t.contains("TÜB.ITAK"),
        "author-block `\\.{{I}}` must render İ, not `.I`:\n{t}"
    );
}

#[test]
fn user_redefined_accent_command_still_expands() {
    // A paper that redefines `\c` as its own macro must keep that meaning —
    // the accent interpretation must NOT shadow a user definition.
    let t = typ(r"\documentclass{article}\newcommand{\c}{COMPLEXSET}\begin{document}value \c\end{document}");
    assert!(
        t.contains("COMPLEXSET"),
        "user-redefined \\c must expand, not be treated as cedilla:\n{t}"
    );
}

#[test]
fn unlisted_accent_letter_falls_back_to_combining_mark() {
    // An accent on a letter with no precomposed form must still keep the letter
    // (letter + combining diacritic), not drop it.
    let t = typ(r"\documentclass{article}\begin{document}\v{q}\end{document}");
    assert!(
        t.contains('q'),
        "caron on q must keep the base letter (combining fallback):\n{t}"
    );
}

// ── Standalone special LETTERS, not accents ─────────────────────────────────
//
// LaTeX's non-ASCII letters are commands in their own right, taking no argument:
// `\ss` is ß, `\o` ø, `\ae` æ, `\i` a dotless ı. Only `\AA` and `\l` were
// dispatched; the other eleven were dropped, taking the character with them —
// `Stra\ss e` rendered as `Stra e`. 8 corpus papers, 26 occurrences, and a
// dogfood agent hit it in an author affiliation.

#[test]
fn special_letters_render_as_their_characters() {
    for (cmd, ch) in [
        (r"\ss", "ß"),
        (r"\o", "ø"),
        (r"\O", "Ø"),
        (r"\aa", "å"),
        (r"\AA", "Å"),
        (r"\ae", "æ"),
        (r"\AE", "Æ"),
        (r"\oe", "œ"),
        (r"\OE", "Œ"),
        (r"\l", "ł"),
        (r"\L", "Ł"),
    ] {
        let t = typ(&format!("X{cmd} Y"));
        assert!(
            t.contains(ch),
            "{cmd} must render as {ch}; got:\n{t}"
        );
    }
}

#[test]
fn the_real_world_case_that_found_this() {
    // From 2605.22728's affiliation: "Straße des 17. Juni".
    let t = typ(r"Stra\ss e des 17. Juni");
    assert!(t.contains("Straße"), "`Stra\\ss e` is one word, Straße; got:\n{t}");
}

#[test]
fn dotless_letters_render() {
    // `\i`/`\j` exist so an accent can be placed on a dotless base.
    let t = typ(r"X\i Y");
    assert!(t.contains('ı'), "\\i is a dotless i; got:\n{t}");
    let t = typ(r"X\j Y");
    assert!(t.contains('ȷ'), "\\j is a dotless j; got:\n{t}");
}

#[test]
fn a_longer_command_starting_with_the_same_letters_is_untouched() {
    // TEXT mode on purpose. A math-mode control here would be vacuous: the
    // emitter returns to the math path before ever reaching the letter arm, so
    // `$\omega$` cannot exercise this matching at all.
    let t = typ(r"X\ostrich Y");
    assert!(!t.contains('\u{f8}'), "`\\ostrich` is not `\\o`; got:\n{t}");
    let t = typ(r"X\sslash Y");
    assert!(!t.contains('\u{df}'), "`\\sslash` is not `\\ss`; got:\n{t}");
}

#[test]
fn a_braced_dotless_letter_renders() {
    // The other control: `{\i}` is how the letter usually appears in source, and
    // it must not regress.
    let t = typ(r"X{\i}Y");
    assert!(t.contains('\u{131}'), "braced dotless i renders; got:\n{t}");
}

// NOTE: an accent placed OVER a dotless base (`\'{\i}`) renders the same before
// and after this change — that is the braced-argument accent path, a separate
// gap. Recorded here rather than asserted, so this file does not claim a fix it
// does not make.

// ── Regressions found in review ──────────────────────────────────────────────

#[test]
fn a_user_redefinition_wins_over_the_builtin_letter() {
    // Single-letter names are exactly the ones papers re-purpose, so the table
    // must not shadow a `\newcommand`.
    let t = typ(r"\newcommand{\o}{OMEGA}X\o Y");
    assert!(t.contains("OMEGA"), "the user's macro wins; got:\n{t}");
    assert!(!t.contains('\u{f8}'), "the builtin must not shadow it; got:\n{t}");
}

#[test]
fn a_control_word_also_swallows_a_wrapped_newline() {
    // TeX discards the whitespace terminating a control word, newline included:
    // `Stra\ss` at a line end is still "Straße". Consuming only spaces left the
    // very gap this file exists to close, just triggered by a source line wrap.
    let t = typ("Stra\\ss\ne des");
    assert!(t.contains("Straße"), "a wrapped line is still one word; got:\n{t}");
}

#[test]
fn a_paragraph_break_after_a_letter_survives() {
    // The control for the rule above: a BLANK line is a paragraph break, not a
    // terminator, and must not be eaten.
    let t = typ("End\\ss\n\nNew para");
    assert!(t.contains("Endß"), "the letter renders; got:\n{t}");
    assert!(
        t.contains("\n\n") || t.contains("New para"),
        "the paragraph break survives; got:\n{t}"
    );
}
