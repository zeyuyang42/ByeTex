//! `\text{...}` inside math copied its argument verbatim into a Typst string, so
//! LaTeX spacing commands survived as literal backslashes in the rendered page.
//!
//! Found by the dogfood loop on 2605.22728 (2026-08-14), where every one of 40
//! `\text{ a.e.\ in }\Omega` labels rendered as `a.e.\ inΩ` — a visible
//! backslash mid-sentence AND the next symbol glued on. Corpus-wide: 24 papers,
//! 197 occurrences. Invisible to `byetex diagnose`, whose leak scan matches
//! `\command` patterns in the BODY and never looks inside an emitted string.
//!
//! Two defects in one place:
//!   * `\ `, `\,`, `\;`, `\:`, `\quad`, … are LaTeX spacing commands; inside a
//!     Typst string they are literal characters, not spacing.
//!   * the argument was `.trim()`ed, discarding LaTeX's meaningful leading and
//!     trailing spaces — which is what glued `\Omega` onto the closing quote.

use byetex_core::{convert, ConvertOptions};

fn typ(src: &str) -> String {
    convert(
        src,
        &ConvertOptions {
            source_name: Some("inline".into()),
            ..Default::default()
        },
    )
    .typst
}

#[test]
fn a_control_space_inside_text_becomes_a_real_space() {
    let out = typ(r"$x = 1 \quad \text{ a.e.\ in }\Omega$");
    assert!(
        !out.contains(r"\\"),
        "no literal backslash may survive into the string; got:\n{out}"
    );
    // Exact, not either-or: the edge spaces are part of the fix, so accepting
    // the trimmed form too would let a regression through.
    assert!(
        out.contains("\" a.e. in \""),
        "the control space should render as a space; got:\n{out}"
    );
}

#[test]
fn the_trailing_space_is_preserved_so_the_next_symbol_is_not_glued() {
    // `\text{ a.e. in }\Omega` renders with a space before Ω in LaTeX; trimming
    // produced `"a.e. in"Omega`.
    // The space must live INSIDE the Typst string — `" a.e. in "Omega` renders
    // "a.e. in Ω", while the old trimmed `"a.e. in"Omega` rendered "a.e. inΩ".
    // (Verified against a real typst render.)
    let out = typ(r"$x \text{ a.e. in }\Omega$");
    assert!(
        out.contains("\" a.e. in \""),
        "LaTeX's edge spaces must be kept inside the string; got:\n{out}"
    );
}

#[test]
fn the_other_latex_spacing_commands_are_converted_too() {
    // Positive spacers must produce a real space; the negative ones must not —
    // asserting only "no backslash" would pass for a spacer silently dropped.
    for cmd in [r"\,", r"\;", r"\:", r"\quad", r"\qquad"] {
        let out = typ(&format!("$x \\text{{a{cmd}b}}$"));
        assert!(
            !out.contains(r"\\"),
            "{cmd} inside \\text must not leave a literal backslash; got:\n{out}"
        );
        assert!(
            out.contains("\"a b\""),
            "{cmd} should become a space; got:\n{out}"
        );
    }
    // `\!` is NEGATIVE space — it separates nothing.
    let neg = typ(r"$x \text{a\!b}$");
    assert!(neg.contains("\"ab\""), "\\! must not add a space; got:\n{neg}");

    // A `\\` line break inside \text degrades to a space rather than rendering
    // two literal backslashes (review finding).
    let br = typ(r"$x \text{a\\b}$");
    assert!(
        !br.contains(r"\\") && br.contains("\"a b\""),
        "a line break inside \\text must not leave backslashes; got:\n{br}"
    );
}

#[test]
fn mbox_gets_the_same_treatment() {
    // `\mbox` routes through the same text path and had the same defect.
    let out = typ(r"$x \mbox{a\ b}$");
    assert!(
        !out.contains(r"\ b"),
        "\\mbox must convert spacing commands too; got:\n{out}"
    );
}

#[test]
fn ordinary_text_is_unchanged() {
    // Guard: the common case must not gain stray spaces or lose characters.
    let out = typ(r"$x = y \text{for all} z$");
    assert!(
        out.contains("\"for all\""),
        "plain text must round-trip unchanged; got:\n{out}"
    );
}

#[test]
fn a_quote_inside_text_does_not_break_the_string() {
    // A `"` in the argument would otherwise terminate the Typst string early.
    let out = typ("$x \\text{say \"hi\"}$");
    assert!(
        !out.contains("\"say \"hi\"\""),
        "an inner quote must be escaped, not left to close the string; got:\n{out}"
    );
}

#[test]
fn mathrm_is_math_mode_and_still_trims() {
    // Review finding: `\mathrm`/`\mathnormal` share this emit path but are MATH
    // mode, where LaTeX ignores whitespace — `\frac{\mathrm{ d }y}{\mathrm{ d }x}`
    // renders "dy/dx". Keeping the edge spaces printed "d y / d x", and would
    // also let source line-wrapping leak indentation into the output.
    let out = typ(r"$\frac{\mathrm{ d }y}{\mathrm{ d }x}$");
    assert!(
        out.contains("\"d\"") && !out.contains("\" d \""),
        "\\mathrm must still trim; got:\n{out}"
    );
}

#[test]
fn the_inner_math_path_keeps_its_separating_space() {
    // Review finding: the `$…$`-splitting path discarded whitespace-only runs,
    // so `\text{ $y$ }\Omega` still glued the neighbour.
    let out = typ(r"$x \text{ $y$ }\Omega$");
    assert!(
        out.contains("\" \""),
        "a whitespace-only run IS the separator; got:\n{out}"
    );
}
