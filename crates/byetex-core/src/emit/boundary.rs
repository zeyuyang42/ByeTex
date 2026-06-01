//! Character-class boundary predicates for preventing math-identifier fusion.
//!
//! Typst math fuses adjacent identifier characters: `alpha` + `beta` printed
//! back to back becomes the single ident `alphabeta`, and `approx` + `22`
//! becomes `approx22`. The emitter prevents this with three mechanisms that
//! historically each re-derived their own `is_ascii_*` checks inline:
//!
//! 1. **Leading guard** ([`ensure_math_letter_boundary`]) — inserts a space
//!    *before* a token when the preceding output and the new token both end/
//!    start with a **letter**. Digits are intentionally excluded: `x2` is a
//!    valid single token, so a digit boundary needs no space here.
//! 2. **Trailing sentinel** ([`needs_trailing_sentinel`]) — marks a spot after
//!    a word-like token so the resolver can later decide whether a space is
//!    needed. Keys on **alphanumeric**, because `approx` *or* `R2` can fuse
//!    with a following letter or digit.
//! 3. **Digit-fusion split** ([`starts_with_digit`]) — when a split letter-run
//!    is followed by a digit (`x2`), a space keeps them apart.
//!
//! These functions are the single vocabulary those mechanisms now share. The
//! letter-vs-alphanumeric distinction is deliberate and load-bearing — keep the
//! two char classes (`is_letter` vs `is_word_char`) separate at every call site.

/// A char that participates in the **leading** letter-boundary guard: an ASCII
/// letter. Digits are excluded on purpose (see module docs).
pub(crate) fn is_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// A char that makes a token "word-like" for **trailing** fusion: ASCII
/// alphanumeric. Both `approx` (letter) and `R2` (digit) can fuse forward.
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

/// Whether `s` begins with a [letter](is_letter).
pub(crate) fn starts_with_letter(s: &str) -> bool {
    s.chars().next().is_some_and(is_letter)
}

/// Whether `s` ends with a [letter](is_letter).
pub(crate) fn ends_with_letter(s: &str) -> bool {
    s.chars().last().is_some_and(is_letter)
}

/// Whether `s` ends with a [word char](is_word_char).
pub(crate) fn ends_with_word_char(s: &str) -> bool {
    s.chars().last().is_some_and(is_word_char)
}

/// Whether `s` begins with an ASCII digit (used by the digit-fusion split).
pub(crate) fn starts_with_digit(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_digit())
}

/// Whether emitting `s` should be followed by a `MATH_WORD_BOUNDARY` sentinel.
///
/// A token that ends in a [word char](is_word_char) may fuse with whatever
/// comes next. `require_multichar` captures the one real difference between the
/// two callers: `push_math_symbol` only marks multi-character symbols (a single
/// emitted letter is already covered by the leading guard), while
/// `emit_subscript` marks even a single bare letter (Bug #33).
pub(crate) fn needs_trailing_sentinel(s: &str, require_multichar: bool) -> bool {
    (!require_multichar || s.chars().count() > 1) && ends_with_word_char(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_letter_is_ascii_alphabetic_only() {
        assert!(is_letter('a'));
        assert!(is_letter('Z'));
        assert!(!is_letter('2'), "digits are not letters for the leading guard");
        assert!(!is_letter('.'));
        assert!(!is_letter('('));
        assert!(!is_letter('é'), "non-ASCII letters are excluded (ASCII-only)");
    }

    #[test]
    fn is_word_char_is_ascii_alphanumeric() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
        assert!(is_word_char('2'), "digits ARE word chars for trailing fusion");
        assert!(!is_word_char('.'));
        assert!(!is_word_char('('));
        assert!(!is_word_char('é'));
    }

    #[test]
    fn starts_with_letter_only_on_leading_letter() {
        assert!(starts_with_letter("alpha"));
        assert!(!starts_with_letter("2x"), "leading digit is not a letter");
        assert!(!starts_with_letter("(x)"));
        assert!(!starts_with_letter(""));
    }

    #[test]
    fn ends_with_letter_only_on_trailing_letter() {
        assert!(ends_with_letter("alpha"));
        assert!(!ends_with_letter("x2"), "trailing digit is not a letter");
        assert!(!ends_with_letter("f("));
        assert!(!ends_with_letter(""));
    }

    #[test]
    fn ends_with_word_char_includes_digits() {
        assert!(ends_with_word_char("approx"));
        assert!(ends_with_word_char("R2"), "trailing digit is a word char");
        assert!(!ends_with_word_char("dot.c."), "trailing dot is not a word char");
        assert!(!ends_with_word_char("f("));
        assert!(!ends_with_word_char(""));
    }

    #[test]
    fn starts_with_digit_only_on_leading_digit() {
        assert!(starts_with_digit("2x"));
        assert!(!starts_with_digit("x2"));
        assert!(!starts_with_digit(""));
    }

    #[test]
    fn needs_trailing_sentinel_multichar_required() {
        // push_math_symbol semantics: require_multichar = true
        assert!(needs_trailing_sentinel("approx", true));
        assert!(needs_trailing_sentinel("R2", true));
        assert!(
            !needs_trailing_sentinel("a", true),
            "single char is covered by the leading guard, no sentinel"
        );
        assert!(
            !needs_trailing_sentinel("dot.c.", true),
            "trailing dot does not fuse"
        );
    }

    #[test]
    fn needs_trailing_sentinel_single_letter_allowed() {
        // emit_subscript semantics (Bug #33): require_multichar = false
        assert!(
            needs_trailing_sentinel("h", false),
            "a bare single-letter subscript still needs the sentinel"
        );
        assert!(needs_trailing_sentinel("approx", false));
        assert!(!needs_trailing_sentinel("f(", false));
        assert!(!needs_trailing_sentinel("", false));
    }
}
