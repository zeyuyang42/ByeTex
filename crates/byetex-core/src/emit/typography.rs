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
        // Dot above (\.)
        ('.', 'I') => Some('İ'), // Turkish dotted capital I (U+0130)
        ('.', 'c') => Some('ċ'),
        ('.', 'C') => Some('Ċ'),
        ('.', 'e') => Some('ė'),
        ('.', 'E') => Some('Ė'),
        ('.', 'g') => Some('ġ'),
        ('.', 'G') => Some('Ġ'),
        ('.', 'z') => Some('ż'),
        ('.', 'Z') => Some('Ż'),
        // Macron (\=)
        ('=', 'a') => Some('ā'),
        ('=', 'A') => Some('Ā'),
        ('=', 'e') => Some('ē'),
        ('=', 'E') => Some('Ē'),
        ('=', 'i') => Some('ī'),
        ('=', 'I') => Some('Ī'),
        ('=', 'o') => Some('ō'),
        ('=', 'O') => Some('Ō'),
        ('=', 'u') => Some('ū'),
        ('=', 'U') => Some('Ū'),
        // Caron / háček (\v)
        ('v', 'c') => Some('č'),
        ('v', 'C') => Some('Č'),
        ('v', 's') => Some('š'),
        ('v', 'S') => Some('Š'),
        ('v', 'z') => Some('ž'),
        ('v', 'Z') => Some('Ž'),
        ('v', 'e') => Some('ě'),
        ('v', 'E') => Some('Ě'),
        ('v', 'r') => Some('ř'),
        ('v', 'R') => Some('Ř'),
        ('v', 'n') => Some('ň'),
        ('v', 'N') => Some('Ň'),
        ('v', 'd') => Some('ď'),
        ('v', 't') => Some('ť'),
        ('v', 'l') => Some('ľ'),
        ('v', 'g') => Some('ǧ'),
        ('v', 'a') => Some('ǎ'),
        ('v', 'A') => Some('Ǎ'),
        // Breve (\u)
        ('u', 'a') => Some('ă'),
        ('u', 'A') => Some('Ă'),
        ('u', 'e') => Some('ĕ'),
        ('u', 'E') => Some('Ĕ'),
        ('u', 'g') => Some('ğ'),
        ('u', 'G') => Some('Ğ'),
        ('u', 'i') => Some('ĭ'),
        ('u', 'o') => Some('ŏ'),
        ('u', 'u') => Some('ŭ'),
        ('u', 'U') => Some('Ŭ'),
        // Double acute (\H)
        ('H', 'o') => Some('ő'),
        ('H', 'O') => Some('Ő'),
        ('H', 'u') => Some('ű'),
        ('H', 'U') => Some('Ű'),
        // Ring above (\r)
        ('r', 'a') => Some('å'),
        ('r', 'A') => Some('Å'),
        ('r', 'u') => Some('ů'),
        ('r', 'U') => Some('Ů'),
        // Cedilla (\c)
        ('c', 'c') => Some('ç'),
        ('c', 'C') => Some('Ç'),
        ('c', 's') => Some('ş'),
        ('c', 'S') => Some('Ş'),
        ('c', 't') => Some('ţ'),
        ('c', 'T') => Some('Ţ'),
        ('c', 'g') => Some('ģ'),
        ('c', 'k') => Some('ķ'),
        ('c', 'l') => Some('ļ'),
        ('c', 'n') => Some('ņ'),
        ('c', 'r') => Some('ŗ'),
        // Ogonek (\k)
        ('k', 'a') => Some('ą'),
        ('k', 'A') => Some('Ą'),
        ('k', 'e') => Some('ę'),
        ('k', 'E') => Some('Ę'),
        ('k', 'i') => Some('į'),
        ('k', 'I') => Some('Į'),
        ('k', 'u') => Some('ų'),
        ('k', 'U') => Some('Ų'),
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
        '.' => Some('\u{0307}'),  // dot above
        '=' => Some('\u{0304}'),  // macron
        'v' => Some('\u{030C}'),  // caron
        'u' => Some('\u{0306}'),  // breve
        'H' => Some('\u{030B}'),  // double acute
        'r' => Some('\u{030A}'),  // ring above
        'c' => Some('\u{0327}'),  // cedilla
        'k' => Some('\u{0328}'),  // ogonek
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
