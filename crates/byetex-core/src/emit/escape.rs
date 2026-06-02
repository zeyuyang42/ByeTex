//! Escape and sanitize helpers for Typst output.
//!
//! These free functions were extracted verbatim from `emit.rs` (pure code
//! motion, no logic change). They translate LaTeX/source fragments into
//! forms safe to splice into Typst markup, math, labels, or table cells.
//! Each encodes a specific, hard-won escaping rule; treat changes as
//! delicate semantic surgery.

/// Escape a plain-text string (author name, affiliation, email, date) for
/// emission into a Typst *content* slot `[...]`. These strings come straight
/// from `parse_authors` / `metadata` as raw text, so — unlike body text — they
/// never pass through the token-level [`needs_text_escape`] path. Without this,
/// an author email like `stas@math.uzh.ch` makes Typst parse `@math.uzh.ch` as
/// a reference (→ "label does not exist"), and a stray `#` / `[` / `<` / `` ` ``
/// terminates or corrupts the slot.
pub(crate) fn escape_text_for_typst_content(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if matches!(
            ch,
            '\\' | '#' | '[' | ']' | '@' | '<' | '`' | '*' | '_' | '$'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// If `kind` is a token that's literal in LaTeX text mode but markup in
/// Typst, return the escaped Typst form. Returns `None` for unaffected kinds.
pub(crate) fn needs_text_escape(kind: &str) -> Option<&'static str> {
    match kind {
        "*" => Some("\\*"),
        "_" => Some("\\_"),
        "[" => Some("\\["),
        "]" => Some("\\]"),
        "#" => Some("\\#"),
        "@" => Some("\\@"),
        "<" => Some("\\<"),
        "`" => Some("\\`"),
        _ => None,
    }
}

/// Replace characters that Typst rejects in `<label>` / `@label`
/// identifiers with `-`. Typst label identifiers allow ASCII
/// letters, digits, `_`, `-`, `:`, `.`. LaTeX is more permissive
/// — `+`, `'`, `*`, etc. appear in real arXiv `\bibitem` keys
/// like `DFG+:InverseInequalitiesNonquasiuniform2004` (Bug #42).
/// Apply this symmetrically at BOTH the bibitem-define and
/// citation/ref-use sites so the rewritten labels match.
/// Whether `c` is valid inside a Typst `<…>` label identifier. Typst accepts
/// Unicode alphanumerics plus `_ - : .`. This is the single source of truth for
/// both `sanitize_label_key` (which rewrites invalid chars on the definition
/// side) and `post_process_typography`'s `<…>` guard (which decides whether an
/// angle span is a real label or literal text) — the two MUST agree, or a label
/// emitted with e.g. `ö` gets escaped to literal text and its `@ref` dangles.
pub(crate) fn is_typst_label_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | ':' | '.')
}

pub(crate) fn sanitize_label_key(key: &str) -> String {
    key.chars()
        .map(|c| if is_typst_label_char(c) { c } else { '-' })
        .collect()
}

/// Replace `;` characters that sit inside any depth of `(...)` in a
/// math body with `#";"` — Typst's content-block escape that renders
/// the literal semicolon glyph. Outside parens (top-level math),
/// `;` is left alone (Typst treats it as a literal there). The
/// matrix/cases emitters write their own `;` separators *outside*
/// the cell-content sub-buffers, so this pass never disturbs them.
///
/// Already-escaped backslash sequences (`\;`, `\#`, etc.) are
/// preserved unchanged.
pub(crate) fn escape_paren_semicolons(body: &str) -> String {
    // Track whether the enclosing paren-call uses `;` as a row
    // separator (`mat`, `cases`, `vec`). Only escape `;` when we're
    // *not* in such a call — those legitimately need the row syntax.
    fn call_uses_semicolon_as_separator(prefix: &str) -> bool {
        let id: String = prefix
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        matches!(id.as_str(), "mat" | "cases" | "vec")
    }
    let mut out = String::with_capacity(body.len());
    // For each open `(`, push whether `;` inside should be escaped.
    let mut escape_in_paren: Vec<bool> = Vec::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                out.push('\\');
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            '(' => {
                let should_escape = !call_uses_semicolon_as_separator(&out);
                escape_in_paren.push(should_escape);
                out.push('(');
            }
            ')' => {
                escape_in_paren.pop();
                out.push(')');
            }
            ';' if escape_in_paren.last().copied().unwrap_or(false) => {
                // Use the bare quoted string `";"` rather than the
                // `#";"` content-block escape. Both render the
                // literal `;` glyph, but `#";"` (where `#` enters
                // code mode) causes Typst to misparse the *next*
                // math token when it's a `(`-grouping — the parens
                // become a code-mode call arg list, and chars like
                // `^` then surface as invalid (driver: 2605.22728's
                // `#";"(L^(min...))`). `";"` stays in math context.
                out.push_str("\";\"");
            }
            other => {
                out.push(other);
            }
        }
    }
    out
}

/// `[...]` brackets are treated as content-scope boundaries: parens opened
/// inside a `[...]` cannot match parens from the outer scope, and vice versa.
/// This prevents a stray `)` inside a `cases([...])` row from mis-escaping
/// the closing `)` of the outer `cases(...)` call.
pub(crate) fn escape_unbalanced_math_brackets(body: &str) -> String {
    let bytes = body.as_bytes();
    // paren_opens: stack of positions of unclosed `(`.
    // bracket_paren_depths: for each open `[`, the paren stack depth at entry.
    //   When `]` closes a `[`, any parens still open inside that scope are
    //   unmatched (the `[...]` boundary prevents them from matching outside).
    let mut paren_opens: Vec<usize> = Vec::new();
    let mut bracket_paren_depths: Vec<usize> = Vec::new();
    let mut escapes: Vec<usize> = Vec::new();
    let mut unmatched_brackets: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        // Skip any backslash-escaped character (including pre-existing
        // `\[` / `\(` from `\left[`-style emissions we may add later).
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        match bytes[i] {
            b'[' => {
                bracket_paren_depths.push(paren_opens.len());
                unmatched_brackets.push(i);
            }
            b']' => {
                if let Some(depth_at_entry) = bracket_paren_depths.pop() {
                    unmatched_brackets.pop();
                    while paren_opens.len() > depth_at_entry {
                        escapes.push(paren_opens.pop().unwrap());
                    }
                } else {
                    escapes.push(i);
                }
            }
            b'(' => paren_opens.push(i),
            b')' => {
                let floor = bracket_paren_depths.last().copied().unwrap_or(0);
                if paren_opens.len() > floor {
                    paren_opens.pop();
                } else {
                    escapes.push(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    // Unclosed `(` and `[` remaining after the scan.
    escapes.extend(unmatched_brackets);
    escapes.extend(paren_opens);
    if escapes.is_empty() {
        return body.to_string();
    }
    escapes.sort_unstable();
    let mut out = String::with_capacity(body.len() + escapes.len());
    let mut last = 0;
    for pos in escapes {
        out.push_str(&body[last..pos]);
        out.push('\\');
        out.push(bytes[pos] as char);
        last = pos + 1;
    }
    out.push_str(&body[last..]);
    out
}

/// Scan `content` from the end for a trailing Typst label `<key>` that was
/// emitted by a nested `\label{...}` inside a theorem environment body.
/// Returns `(cleaned_content, Some(key))` if a valid label is found at the
/// end of the content, or `(content, None)` otherwise.
///
/// Valid label keys: start with a letter or digit, contain only
/// `[a-zA-Z0-9:_.-]`.  The label must NOT be immediately preceded by `$`
/// or `)` (which would indicate it belongs to a nested equation or figure,
/// not the theorem itself).
pub(crate) fn strip_trailing_typst_label(content: &str) -> (String, Option<String>) {
    let trimmed = content.trim_end();
    if !trimmed.ends_with('>') {
        return (content.to_string(), None);
    }
    let close = trimmed.len() - 1; // index of '>'
    let open = match trimmed[..close].rfind('<') {
        Some(i) => i,
        None => return (content.to_string(), None),
    };
    let key = &trimmed[open + 1..close];
    // Use the same valid-label-char test as `sanitize_label_key` / the
    // typography guard (Unicode-aware) so a hoisted theorem label with a
    // non-ASCII letter is recognised consistently.
    if key.is_empty()
        || !key.starts_with(|c: char| c.is_alphanumeric())
        || !key.chars().all(is_typst_label_char)
    {
        return (content.to_string(), None);
    }
    // Don't hoist labels that belong to a preceding equation (`$…$`) or figure (`)`)
    let before = trimmed[..open].trim_end();
    if before.ends_with('$') || before.ends_with(')') || before.ends_with(']') {
        return (content.to_string(), None);
    }
    (before.to_string(), Some(key.to_string()))
}

/// Escape the handful of ASCII characters that have special meaning
/// inside a Typst `[...]` content block but commonly appear unescaped
/// in table cells / caption text: `_` opens italic, `*` opens bold,
/// `#` opens code context, `<` opens labels, `@` opens references.
/// Already-escaped `\_` / `\*` / etc. are left alone so this is
/// idempotent. We don't touch `[`, `]`, `{`, `}` here — the caller
/// already balances those, and over-escaping breaks the surrounding
/// content block.
pub(crate) fn escape_text_cell(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut in_math = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // `#raw(...)` is Typst code, not markup — copy the whole call verbatim
        // so markup escaping never touches the raw string literal (e.g. an
        // underscore inside `#raw("a_b")` must stay `a_b`, not become `a\_b`,
        // which is not a valid Typst string escape). Track string literals so a
        // `)` inside the content doesn't end the call early.
        if !in_math && chars[i..].starts_with(&['#', 'r', 'a', 'w', '(']) {
            out.push_str("#raw(");
            i += 5;
            let mut depth = 1usize;
            let mut in_str = false;
            while i < chars.len() && depth > 0 {
                let ch = chars[i];
                if in_str {
                    out.push(ch);
                    if ch == '\\' && i + 1 < chars.len() {
                        out.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    if ch == '"' {
                        in_str = false;
                    }
                    i += 1;
                } else {
                    match ch {
                        '"' => in_str = true,
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        _ => {}
                    }
                    out.push(ch);
                    i += 1;
                }
            }
            continue;
        }
        match c {
            '$' => {
                in_math = !in_math;
                out.push('$');
            }
            '\\' => {
                // Preserve any backslash-escape verbatim — `\_` etc.
                out.push('\\');
                if i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 1;
                }
            }
            '#' => {
                if in_math
                    || chars
                        .get(i + 1)
                        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
                {
                    out.push('#');
                } else {
                    out.push_str("\\#");
                }
            }
            // These are Typst markup operators in TEXT, but all valid verbatim
            // inside math (`_`/`^` attachments, `*` superscript star, `<`
            // relation, `` ` `` …). The cell's `$...$` content is already
            // converted Typst math, so escaping here would corrupt it
            // (`$y_w$` → `$y\_w$`, a literal underscore, not a subscript).
            '_' | '*' | '@' | '<' | '`' if !in_math => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}
