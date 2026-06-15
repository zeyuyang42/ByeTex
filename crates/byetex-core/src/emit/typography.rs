//! Text-accent precomposition and math-word recognition helpers,
//! extracted from emit.rs. Pure functions over strings/chars (no `Emitter`).

/// Map a LaTeX text accent + base letter to the precomposed Unicode codepoint.
///
/// `accent` is the accent character: `'\''` acute, '`' grave, `'"'` diaeresis,
/// `'^'` circumflex, `'~'` tilde. Returns a `String` so the combining-mark
/// fallback path (two code points) is representable.
pub(crate) fn apply_text_accent(accent: char, letter: char) -> String {
    let precomposed: Option<char> = match (accent, letter) {
        // Acute (')
        ('\'', 'a') => Some('á'),
        ('\'', 'A') => Some('Á'),
        ('\'', 'e') => Some('é'),
        ('\'', 'E') => Some('É'),
        ('\'', 'i') => Some('í'),
        ('\'', 'I') => Some('Í'),
        ('\'', 'o') => Some('ó'),
        ('\'', 'O') => Some('Ó'),
        ('\'', 'u') => Some('ú'),
        ('\'', 'U') => Some('Ú'),
        ('\'', 'y') => Some('ý'),
        ('\'', 'Y') => Some('Ý'),
        ('\'', 'n') => Some('ń'),
        ('\'', 'N') => Some('Ń'),
        ('\'', 'c') => Some('ć'),
        ('\'', 'C') => Some('Ć'),
        ('\'', 's') => Some('ś'),
        ('\'', 'S') => Some('Ś'),
        ('\'', 'z') => Some('ź'),
        ('\'', 'Z') => Some('Ź'),
        ('\'', 'l') => Some('ĺ'),
        ('\'', 'L') => Some('Ĺ'),
        ('\'', 'r') => Some('ŕ'),
        ('\'', 'R') => Some('Ŕ'),
        // Grave (`)
        ('`', 'a') => Some('à'),
        ('`', 'A') => Some('À'),
        ('`', 'e') => Some('è'),
        ('`', 'E') => Some('È'),
        ('`', 'i') => Some('ì'),
        ('`', 'I') => Some('Ì'),
        ('`', 'o') => Some('ò'),
        ('`', 'O') => Some('Ò'),
        ('`', 'u') => Some('ù'),
        ('`', 'U') => Some('Ù'),
        ('`', 'n') => Some('ǹ'),
        ('`', 'N') => Some('Ǹ'),
        // Diaeresis (")
        ('"', 'a') => Some('ä'),
        ('"', 'A') => Some('Ä'),
        ('"', 'e') => Some('ë'),
        ('"', 'E') => Some('Ë'),
        ('"', 'i') => Some('ï'),
        ('"', 'I') => Some('Ï'),
        ('"', 'o') => Some('ö'),
        ('"', 'O') => Some('Ö'),
        ('"', 'u') => Some('ü'),
        ('"', 'U') => Some('Ü'),
        ('"', 'y') => Some('ÿ'),
        ('"', 'Y') => Some('Ÿ'),
        // Circumflex (^)
        ('^', 'a') => Some('â'),
        ('^', 'A') => Some('Â'),
        ('^', 'e') => Some('ê'),
        ('^', 'E') => Some('Ê'),
        ('^', 'i') => Some('î'),
        ('^', 'I') => Some('Î'),
        ('^', 'o') => Some('ô'),
        ('^', 'O') => Some('Ô'),
        ('^', 'u') => Some('û'),
        ('^', 'U') => Some('Û'),
        ('^', 'c') => Some('ĉ'),
        ('^', 'C') => Some('Ĉ'),
        ('^', 'g') => Some('ĝ'),
        ('^', 'G') => Some('Ĝ'),
        ('^', 'h') => Some('ĥ'),
        ('^', 'H') => Some('Ĥ'),
        ('^', 'j') => Some('ĵ'),
        ('^', 'J') => Some('Ĵ'),
        ('^', 's') => Some('ŝ'),
        ('^', 'S') => Some('Ŝ'),
        ('^', 'w') => Some('ŵ'),
        ('^', 'W') => Some('Ŵ'),
        ('^', 'y') => Some('ŷ'),
        ('^', 'Y') => Some('Ŷ'),
        // Tilde (~)
        ('~', 'a') => Some('ã'),
        ('~', 'A') => Some('Ã'),
        ('~', 'e') => Some('ẽ'),
        ('~', 'E') => Some('Ẽ'),
        ('~', 'i') => Some('ĩ'),
        ('~', 'I') => Some('Ĩ'),
        ('~', 'n') => Some('ñ'),
        ('~', 'N') => Some('Ñ'),
        ('~', 'o') => Some('õ'),
        ('~', 'O') => Some('Õ'),
        ('~', 'u') => Some('ũ'),
        ('~', 'U') => Some('Ũ'),
        _ => None,
    };
    if let Some(c) = precomposed {
        return c.to_string();
    }
    // Combining-mark fallback: letter + Unicode combining diacritic.
    let combining: Option<char> = match accent {
        '\'' => Some('\u{0301}'),
        '`' => Some('\u{0300}'),
        '"' => Some('\u{0308}'),
        '^' => Some('\u{0302}'),
        '~' => Some('\u{0303}'),
        _ => None,
    };
    let mut s = letter.to_string();
    if let Some(m) = combining {
        s.push(m);
    }
    s
}

/// Decide whether a word inside math should be split into single characters.
/// LaTeX semantics: consecutive letters are implicit products (`mc` = m·c).
/// Typst semantics: consecutive letters form an identifier (`mc` = variable mc).
/// We split iff the word is more than one ASCII letter long and is not a
/// recognized math function name.
pub(in crate::emit) fn should_split_math_word(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    if !bytes.iter().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    if is_math_function_name(s) {
        return false;
    }
    true
}

/// LaTeX math operators that Typst does NOT provide as built-in upright
/// identifiers (sin/cos/… exist; these don't), so a bare `cov` parses as an
/// unknown variable. Emitted via `op("…")` (upright, like `\operatorname`).
/// They are also "function names" for the no-split rule.
pub(in crate::emit) fn is_operatorname_only_function(s: &str) -> bool {
    matches!(s, "cov" | "var" | "argmax" | "argmin")
}

/// Common LaTeX math functions that Typst also renders as upright identifiers.
/// Words matching these don't get character-split.
pub(in crate::emit) fn is_math_function_name(s: &str) -> bool {
    matches!(
        s,
        "sin"
            | "cos"
            | "tan"
            | "cot"
            | "sec"
            | "csc"
            | "arcsin"
            | "arccos"
            | "arctan"
            | "sinh"
            | "cosh"
            | "tanh"
            | "log"
            | "ln"
            | "exp"
            | "min"
            | "max"
            | "inf"
            | "sup"
            | "lim"
            | "det"
            | "arg"
            | "deg"
            | "dim"
            | "gcd"
            | "hom"
            | "ker"
            | "lg"
            | "mod"
            | "Pr"
            | "Re"
            | "Im"
            | "argmin"
            | "argmax"
            | "limsup"
            | "liminf"
            | "var"
            | "cov"
    )
}
