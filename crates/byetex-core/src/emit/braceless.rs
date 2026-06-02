//! Brace-less LaTeX argument consumption + macro-arg substitution, extracted from emit.rs (pure code motion).

/// Extract the class name and option list from a `class_include` node.
/// `\documentclass[opt1,opt2]{class}` → (Some("class"), ["opt1", "opt2"]).
/// Pull `(\name, MacroDef)` out of a `new_command_definition` node.
/// AST shape (`\newcommand{\name}[N]{body}`):
///
/// ```text
/// new_command_definition
///   \newcommand                      (literal)
///   curly_group_command_name         contains `{ command_name "\\name" }`
///   brack_group_argc (optional)      contains `[ argc "N" ]`
///   brack_group (optional, skipped)  the optional-default form — unsupported
///   curly_group                      the macro body
/// ```
/// The three shapes a brace-less LaTeX argument can take. See
/// [`consume_braceless_arg`].
#[derive(Debug, Clone)]
pub(crate) enum BracelessArg {
    /// A `\command-name` (with the leading backslash). Letters-only run;
    /// for single-character escapes like `\%` or `\é` the next char is
    /// included regardless of class.
    Command(String),
    /// The inner content of a balanced `{...}` group, sans braces.
    Group(String),
    /// A single Unicode codepoint argument (letter, digit, punctuation).
    Char(String),
}

impl BracelessArg {
    /// The textual representation used as a substitution body for
    /// `\newcommand` expansion. For `Command` this is the literal
    /// `\name`; for `Group` it's the inner content; for `Char` it's the
    /// single codepoint.
    pub(crate) fn as_substitution(&self) -> &str {
        match self {
            BracelessArg::Command(s) | BracelessArg::Group(s) | BracelessArg::Char(s) => s,
        }
    }
}

// ─── Braceless-arg & macro machinery ──────────────────────────────────────────

/// Consume one LaTeX argument starting at byte offset `start` in `src`,
/// LaTeX-style: leading ASCII whitespace is skipped, then the next token
/// is read as either a `\command` run, a balanced `{group}`, or one
/// Unicode codepoint.
///
/// Returns `Some((arg, end_byte))` on success, where `end_byte` is the
/// byte index immediately past the consumed token. Returns `None` only
/// when `start` lies past EOF or the remaining bytes are pure whitespace
/// — the caller decides whether that's an error condition.
///
/// Used by both [`Emitter::emit_math_wrap`] (math accents like `\hat x`,
/// `\bar\alpha`, `\mathbf{X}`) and [`Emitter::expand_user_macro`] so
/// `\newcommand`s called brace-less (`\mat X`, `\rvec\alpha`) work the
/// same way LaTeX expects.
/// Math-context wrapper around [`consume_braceless_arg`] that refuses
/// to consume a math-terminating delimiter (`$`, `\)`, `\]`, or `}`
/// at the outer level). Used by structural math commands (`\frac`,
/// `\sqrt`, `\binom`) when filling missing brace-less args: without
/// this guard, `$\frac{a}$` would greedily eat the closing `$` as the
/// second argument and break the surrounding math container.
pub(crate) fn try_consume_math_arg(src: &str, start: usize) -> Option<(BracelessArg, usize)> {
    let bytes = src.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    if bytes[i] == b'$' || bytes[i] == b'}' {
        // Math closer (`$`, `$$`) or surrounding-group closer. Bail.
        return None;
    }
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
        match bytes[i + 1] {
            b')' | b']' => return None, // `\)` / `\]` math closers
            _ => {}
        }
        // `\end{...}` — math environment closer.
        if src[i..].starts_with("\\end{") {
            return None;
        }
    }
    consume_braceless_arg(src, start)
}

/// Starting at `start`, skip leading whitespace then consume zero or more
/// consecutive balanced `{...}` argument groups, returning the byte index past
/// the last one (or `start` if none follow). Used to gather the brace args of a
/// structural command that was consumed brace-less, e.g. the `{a}{b}` of
/// `\sqrt\frac{a}{b}`.
pub(in crate::emit) fn consume_trailing_brace_groups(src: &str, start: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = start;
    loop {
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'{' {
            return i;
        }
        let mut depth = 1i32;
        let mut k = j + 1;
        while k < bytes.len() {
            match bytes[k] {
                b'\\' if k + 1 < bytes.len() => {
                    k += 2;
                    continue;
                }
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            k += 1;
        }
        if k >= bytes.len() {
            // Unbalanced — stop at what we had.
            return i;
        }
        i = k + 1;
    }
}

pub(crate) fn consume_braceless_arg(src: &str, start: usize) -> Option<(BracelessArg, usize)> {
    let bytes = src.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
        // `\name` — ASCII-letter run, OR single-char escape (`\%`, `\é`).
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
            j += 1;
        }
        if j == i + 1 {
            // Single-char escape. Advance by codepoint length so we
            // never split a multi-byte UTF-8 sequence mid-byte.
            let after = &src[i + 1..];
            let step = after.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            j = i + 1 + step;
        }
        return Some((BracelessArg::Command(src[i..j].to_string()), j));
    }
    if bytes[i] == b'{' {
        // Balanced `{...}` group; depth-track, ignore `\{` and `\}`.
        let inner_start = i + 1;
        let mut depth = 1i32;
        let mut j = inner_start;
        while j < bytes.len() {
            match bytes[j] {
                b'\\' if j + 1 < bytes.len() => {
                    j += 2;
                    continue;
                }
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        if j >= bytes.len() {
            // Unbalanced — fail closed so the caller can warn.
            return None;
        }
        return Some((BracelessArg::Group(src[inner_start..j].to_string()), j + 1));
    }
    // Single Unicode codepoint.
    let rest = &src[i..];
    let c = rest.chars().next()?;
    let end = i + c.len_utf8();
    Some((BracelessArg::Char(c.to_string()), end))
}

/// Substitute `#1`..`#N` placeholders in a `\newcommand` body. Walks
/// the body character-by-character so `#10` doesn't accidentally match
/// `#1`+`0` and an unmatched `#<digit>` (outside the param range) is
/// passed through unchanged.
pub(in crate::emit) fn substitute_macro_args(body: &str, args: &[String]) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' {
            // Consume a run of digits and look up the parameter index.
            let mut digits = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    digits.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if digits.is_empty() {
                out.push('#');
            } else if let Ok(idx) = digits.parse::<usize>() {
                // `\newcommand` parameters are 1-indexed.
                if idx >= 1 && idx <= args.len() {
                    out.push_str(&args[idx - 1]);
                } else {
                    // No matching arg — keep the placeholder verbatim.
                    out.push('#');
                    out.push_str(&digits);
                }
            } else {
                out.push('#');
                out.push_str(&digits);
            }
        } else {
            out.push(c);
        }
    }
    out
}
