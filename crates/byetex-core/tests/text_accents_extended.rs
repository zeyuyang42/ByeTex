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
