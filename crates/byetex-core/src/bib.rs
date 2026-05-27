//! BibTeX file preprocessor.
//!
//! Typst's built-in BibLaTeX/Hayagriva parser is strict and rejects two
//! patterns we see in real-world arXiv `.bib` files:
//!
//! 1. **Unresolved `@string` references.** A field like
//!    `Journal = mor,` references the abbreviation `mor` that the
//!    paper expects to be defined by `@string{mor = "..."}`. When
//!    the `@string` isn't present (or Typst can't expand it),
//!    parsing aborts with `unknown abbreviation "mor"`.
//!
//! 2. **Whitespace between `@type{` and the entry key.** Real BibTeX
//!    accepts `@inproceedings{\n  Spliethoever.2025,\n  ...` but
//!    Typst expects the key immediately after `{`. The parse aborts
//!    with `expected identifier`.
//!
//! This module exposes [`preprocess_bib`] which returns a Typst-safe
//! rewriting of the input:
//!
//! - `@string{NAME = "value"}` entries are collected into a map.
//! - Within every other entry's fields, bare-identifier values are:
//!   - replaced with the literal string when the map has a matching
//!     `@string`, OR
//!   - wrapped in double quotes when no match is found (graceful
//!     fallback that keeps the entry parseable instead of crashing
//!     the whole compile — Bibliography quality degrades for that
//!     one field).
//! - `@type{\s+key,` is normalised to `@type{key,` (whitespace
//!   between `{` and the key is dropped).
//!
//! The function is intentionally permissive: comments, `@preamble`,
//! `@comment` blocks pass through unchanged.

use std::collections::HashMap;

/// Rewrite a `.bib` source string so Typst's BibLaTeX parser accepts
/// it. See module docs for the transformations applied.
pub fn preprocess_bib(input: &str) -> String {
    let string_defs = collect_string_defs(input);
    let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = String::with_capacity(input.len() + 64);
    let mut pos = 0usize;
    let bytes = input.as_bytes();
    while pos < bytes.len() {
        // Find the next `@`. Anything before is preserved verbatim
        // (comments outside entries, leading whitespace, etc.).
        let next_at = match input[pos..].find('@') {
            Some(p) => pos + p,
            None => {
                out.push_str(&input[pos..]);
                break;
            }
        };
        out.push_str(&input[pos..next_at]);
        // Identify the entry type.
        let type_start = next_at + 1;
        let type_end = bytes[type_start..]
            .iter()
            .position(|&b| !b.is_ascii_alphabetic())
            .map(|i| type_start + i)
            .unwrap_or(bytes.len());
        let entry_type = input[type_start..type_end].to_ascii_lowercase();
        // Skip whitespace.
        let mut i = type_end;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        // Must see `{` for a valid entry.
        if i >= bytes.len() || bytes[i] != b'{' {
            // If the entry type is empty (bare `@` with no type, e.g. a
            // deleted entry left behind), drop it silently — passing it
            // through would cause Typst's parser to abort with
            // "expected identifier" (paper 2605.22724).
            // Otherwise preserve the `@` verbatim (e.g. `@` inside a
            // comment or preamble text).
            if !entry_type.is_empty() {
                out.push('@');
            }
            pos = next_at + 1;
            continue;
        }
        let body_start = i + 1;
        // Find matching `}` for this entry (brace-aware).
        let body_end = match find_matching_brace(bytes, body_start) {
            Some(e) => e,
            None => {
                // Unbalanced; pass through to end.
                out.push_str(&input[next_at..]);
                break;
            }
        };
        let body = &input[body_start..body_end];
        if entry_type == "string" {
            // Already collected; preserve as-is so Typst sees a valid
            // (if unused) @string block.
            out.push_str(&input[next_at..body_end + 1]);
        } else if entry_type == "preamble" || entry_type == "comment" {
            // Pass through.
            out.push_str(&input[next_at..body_end + 1]);
        } else {
            // Regular entry — rewrite. Drop duplicate-key entries
            // (Typst's parser aborts with `duplicate key "X"` on
            // collisions; 2605.22507's bib has them).
            let trimmed = body.trim_start();
            let key = trimmed
                .find(',')
                .map(|c| trimmed[..c].trim().to_string())
                .unwrap_or_default();
            if !key.is_empty() && !seen_keys.insert(key.clone()) {
                // Already-seen key — skip this entry entirely.
            } else {
                let rewritten = rewrite_entry(&entry_type, body, &string_defs);
                out.push_str(&rewritten);
            }
        }
        pos = body_end + 1;
    }
    out
}

/// Find the matching `}` for an opening `{` at position `start`.
/// `start` should point to the byte AFTER the `{`. Returns the
/// position of the matching `}`. Brace-balanced; doesn't recognise
/// escape sequences (BibTeX uses unescaped braces).
fn find_matching_brace(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1i32;
    let mut in_string = false;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            // Backslash escape: skip the next byte unconditionally so
            // LaTeX accents like `\"o` (umlaut), `\&`, etc. don't toggle
            // string state or affect brace counting.
            b'\\' if i + 1 < bytes.len() => {
                i += 2;
                continue;
            }
            // BibTeX uses both `"..."` and `{...}` as field-value
            // delimiters, but only at the OUTERMOST level of the entry
            // (depth == 1). Inside nested `{...}` groups (depth > 1) a
            // `"` is literal text, never a string-toggle. Restricting
            // the toggle to depth==1 keeps brace counting correct for
            // values like `author = {Splieth{\"o}ver}`.
            b'"' if depth == 1 => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Scan the input for `@string{NAME = "value"}` and collect a
/// case-insensitive name -> value map.
fn collect_string_defs(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let bytes = input.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        let at = match input[pos..].find('@') {
            Some(p) => pos + p,
            None => break,
        };
        let type_start = at + 1;
        let type_end = bytes[type_start..]
            .iter()
            .position(|&b| !b.is_ascii_alphabetic())
            .map(|i| type_start + i)
            .unwrap_or(bytes.len());
        let ty = input[type_start..type_end].to_ascii_lowercase();
        let mut i = type_end;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'{' {
            pos = at + 1;
            continue;
        }
        let body_start = i + 1;
        let body_end = match find_matching_brace(bytes, body_start) {
            Some(e) => e,
            None => break,
        };
        if ty == "string" {
            let body = &input[body_start..body_end];
            // Body is `NAME = "value"` or `NAME = {value}`.
            if let Some(eq) = body.find('=') {
                let name = body[..eq].trim().to_ascii_lowercase();
                let val = body[eq + 1..].trim();
                let unquoted = strip_outer_brace_or_quote(val);
                if !name.is_empty() {
                    map.insert(name, unquoted.to_string());
                }
            }
        }
        pos = body_end + 1;
    }
    map
}

/// Strip a single layer of `"..."` or `{...}` around a value.
fn strip_outer_brace_or_quote(s: &str) -> &str {
    let s = s.trim_end_matches(',').trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('{') && s.ends_with('}') && s.len() >= 2)
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Rewrite a single entry body. `entry_type` is lowercase (e.g.
/// "article", "inproceedings"). `body` is the content between the
/// outer `{` and `}` of the entry. Returns the full `@type{body}`
/// reconstruction.
fn rewrite_entry(entry_type: &str, body: &str, strings: &HashMap<String, String>) -> String {
    // Bug #40 part 1: drop any leading whitespace/newlines between
    // `{` and the entry key. Typst's parser requires the key
    // immediately.
    let trimmed = body.trim_start();
    // Find the entry key (up to the first `,`).
    let comma = match trimmed.find(',') {
        Some(p) => p,
        None => {
            // No comma → no key/fields. Skip entry as malformed.
            return String::new();
        }
    };
    let key = trimmed[..comma].trim();
    if key.is_empty() {
        // Truly malformed — no key. Drop.
        return String::new();
    }
    let fields_src = &trimmed[comma + 1..];
    let mut out = String::with_capacity(body.len() + 16);
    out.push('@');
    out.push_str(entry_type);
    out.push('{');
    out.push_str(key);
    out.push(',');
    out.push_str(&rewrite_fields(fields_src, strings));
    out.push('}');
    out
}

/// Walk a comma-separated field list, replacing bare-identifier
/// values with either the resolved `@string` value or a
/// quote-wrapped form.
///
/// Field shape: `name = value` where value is one of
/// - `"..."` (quoted string — pass through)
/// - `{...}` (braced — pass through)
/// - `123`   (number — pass through)
/// - `name`  (bare identifier — RESOLVE)
///
/// We don't fully parse fields; we just scan for `=` followed by a
/// value run, decide the value's shape, and rewrite if needed.
fn rewrite_fields(src: &str, strings: &HashMap<String, String>) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        // Find next `=` outside strings/braces (no nesting tracking
        // needed since fields are flat between commas).
        let eq = match memmem_outside_groups(bytes, b'=', i) {
            Some(p) => p,
            None => {
                out.push_str(&src[i..]);
                break;
            }
        };
        // Real-world `.bib` files sometimes have a stray `@` glued to
        // a field name (e.g. `@doi = {...}` — line 457 of 22738's
        // bib). Typst's parser then thinks a new entry is starting.
        // Strip the `@` when it's part of a field-name token.
        let seg = strip_stray_at_in_field_names(&src[i..=eq]);
        // Extract the field name for context-sensitive value handling.
        // `src[i..eq]` contains the tail of the previous value (`,`)
        // plus whitespace plus the field-name token; take the last
        // non-whitespace word, then strip any leading `@`.
        let field_name = src[i..eq]
            .split_whitespace()
            .next_back()
            .unwrap_or("")
            .trim_start_matches('@')
            .to_ascii_lowercase();
        out.push_str(&seg);
        let mut j = eq + 1;
        // Skip whitespace after `=`.
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            out.push(bytes[j] as char);
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }
        let (value_text, new_i) = read_field_value(bytes, src, strings, j, &field_name);
        out.push_str(&value_text);
        i = new_i;
    }
    out
}

/// Read a BibTeX field value starting at `pos`.  Handles the BibTeX
/// `#` string-concatenation operator (`"oct" # "-" # nov`), which
/// Typst's BibLaTeX parser does not support.  When `#` is detected
/// the pieces are merged and returned as a single quoted string.
/// Without `#`, the original text is returned for quoted/braced/
/// numeric values (pass-through) while bare identifiers are resolved
/// and quoted as before.
///
/// `field_name` (lowercase) is used for context-sensitive handling:
/// for `month` fields with `#` concatenation (month ranges like
/// `"aug" # "-" # sep`) only the first term is kept because Typst's
/// hayagriva parser rejects range strings like `"aug-sep"`.
///
/// Returns `(emitted_text, new_pos)`.
fn read_field_value(
    bytes: &[u8],
    src: &str,
    strings: &HashMap<String, String>,
    pos: usize,
    field_name: &str,
) -> (String, usize) {
    // Read the first term and peek for `#`.
    let (first_content, first_end, first_raw) = read_bib_term(bytes, src, strings, pos);

    let mut k = first_end;
    while k < bytes.len() && bytes[k].is_ascii_whitespace() {
        k += 1;
    }
    if k >= bytes.len() || bytes[k] != b'#' {
        // Simple value (no concatenation) — return the raw form so
        // quoted/braced/number fields pass through unchanged.
        return (first_raw, first_end);
    }

    // For `month` fields, BibTeX month ranges (`"aug" # "-" # sep`)
    // collapse to strings like "aug-sep" that Typst rejects with
    // "missing number". Keep only the first term and skip the rest
    // of the concatenation chain.
    if field_name == "month" {
        // Consume the full chain so `i` advances past all terms.
        k += 1; // skip `#`
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        while k < bytes.len() {
            let (_, end, _) = read_bib_term(bytes, src, strings, k);
            k = end;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b'#' {
                k += 1;
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
            } else {
                break;
            }
        }
        return (first_raw, k);
    }

    // `#`-concatenation: accumulate all terms into one string, then
    // re-emit as a single quoted value.
    let mut combined = first_content;
    k += 1; // skip `#`
    while k < bytes.len() && bytes[k].is_ascii_whitespace() {
        k += 1;
    }
    while k < bytes.len() {
        let (term, end, _) = read_bib_term(bytes, src, strings, k);
        combined.push_str(&term);
        k = end;
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        if k < bytes.len() && bytes[k] == b'#' {
            k += 1;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
        } else {
            break;
        }
    }
    let mut quoted = String::with_capacity(combined.len() + 2);
    quoted.push('"');
    for c in combined.chars() {
        if c == '"' || c == '\\' {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    (quoted, k)
}

/// Read a single BibTeX value term at `pos`.  Returns
/// `(decoded_content, end_pos, raw_text)`:
/// - `decoded_content`: the inner text (used when concatenating with `#`)
/// - `end_pos`: byte position after the term
/// - `raw_text`: original source text (used for pass-through)
fn read_bib_term(
    bytes: &[u8],
    src: &str,
    strings: &HashMap<String, String>,
    pos: usize,
) -> (String, usize, String) {
    if pos >= bytes.len() {
        return (String::new(), pos, String::new());
    }
    match bytes[pos] {
        b'"' => {
            let end = find_closing(bytes, pos, b'"', b'"');
            let inner = src[pos + 1..end].to_string();
            let raw = src[pos..=end].to_string();
            (inner, end + 1, raw)
        }
        b'{' => {
            let end = find_matching_brace(bytes, pos + 1).unwrap_or(bytes.len() - 1);
            let inner = src[pos + 1..end].to_string();
            let raw = src[pos..=end].to_string();
            (inner, end + 1, raw)
        }
        b'0'..=b'9' => {
            let end = bytes[pos..]
                .iter()
                .position(|&b| b == b',' || b == b'\n')
                .map(|p| pos + p)
                .unwrap_or(bytes.len());
            let text = src[pos..end].to_string();
            (text.clone(), end, text)
        }
        b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
            let id_end = bytes[pos..]
                .iter()
                .position(|&b| {
                    !(b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
                })
                .map(|p| pos + p)
                .unwrap_or(bytes.len());
            let ident = &src[pos..id_end];
            let ident_lc = ident.to_ascii_lowercase();
            let (content, raw) = if let Some(value) = strings.get(&ident_lc) {
                let raw = push_quoted(value);
                (value.clone(), raw)
            } else {
                let raw = push_quoted(ident);
                (ident.to_string(), raw)
            };
            (content, id_end, raw)
        }
        b => (
            (b as char).to_string(),
            pos + 1,
            (b as char).to_string(),
        ),
    }
}

/// Wrap `s` in double-quotes, escaping inner `"` and `\`.
fn push_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Remove stray `@` characters that appear in field-name positions.
/// A field-name position is: after `,` (or at the very start) with
/// only whitespace between. Any other `@` is left alone.
fn strip_stray_at_in_field_names(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_field_start = true;
    for c in s.chars() {
        if c == '\n' || c == ',' {
            out.push(c);
            at_field_start = true;
        } else if c.is_ascii_whitespace() {
            out.push(c);
            // Keep `at_field_start` true.
        } else if c == '@' && at_field_start {
            // Drop the stray `@` — we're at a field-name position.
        } else {
            out.push(c);
            at_field_start = false;
        }
    }
    out
}

/// Find the next occurrence of `target` in `bytes[start..]` that
/// sits outside any `"..."` or `{...}` group.
fn memmem_outside_groups(bytes: &[u8], target: u8, start: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => depth -= 1,
            b if b == target && !in_string && depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Find the byte position of the next `close` byte starting from
/// `start` (which points at the opening `open`). Used for quoted
/// strings where open == close == `"`.
fn find_closing(bytes: &[u8], start: usize, _open: u8, close: u8) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == close {
            return i;
        }
        i += 1;
    }
    bytes.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_at_string_reference() {
        let src = "@string{jcp = \"Journal of Computational Physics\"}\n\
                   @article{foo, journal = jcp, year = 2024}\n";
        let out = preprocess_bib(src);
        assert!(
            out.contains(r#""Journal of Computational Physics""#),
            "got: {}",
            out
        );
        assert!(
            !out.contains("journal = jcp"),
            "old bare ref remains: {}",
            out
        );
    }

    #[test]
    fn quotes_unresolved_bare_identifier() {
        // No @string defined for `mor`.
        let src = "@article{foo, journal = mor, year = 1997}\n";
        let out = preprocess_bib(src);
        assert!(
            out.contains("\"mor\""),
            "expected quoted fallback; got: {}",
            out
        );
    }

    #[test]
    fn normalises_key_whitespace() {
        let src = "@inproceedings{\n    Spliethoever.2025,\n    title = \"Foo\"\n}\n";
        let out = preprocess_bib(src);
        assert!(
            out.contains("@inproceedings{Spliethoever.2025,"),
            "key whitespace not normalised; got:\n{}",
            out
        );
    }

    #[test]
    fn preserves_quoted_and_braced_values() {
        let src = "@article{x, title = \"Hello\", note = {with {nested} braces}, year = 2024}\n";
        let out = preprocess_bib(src);
        assert!(out.contains("title = \"Hello\""), "quoted lost: {}", out);
        assert!(
            out.contains("note = {with {nested} braces}"),
            "braced lost: {}",
            out
        );
        assert!(out.contains("year = 2024"), "number lost: {}", out);
    }

    #[test]
    fn drops_entry_with_no_key() {
        let src = "@inproceedings{, title = \"orphan\"}\n@article{good, year = 2024}\n";
        let out = preprocess_bib(src);
        assert!(
            !out.contains("orphan"),
            "keyless entry not dropped: {}",
            out
        );
        assert!(out.contains("@article{good,"), "good entry lost: {}", out);
    }

    #[test]
    fn strips_stray_at_before_field_name() {
        // Real bug from 2605.22738: `,\n\t@doi = {...}` — the `@`
        // before `doi` makes Typst's parser think a new entry is
        // starting and abort with `expected identifier`.
        let src = "@article{x, title = {Foo}, year = 2024,\n\t@doi = {10.1109/abc}\n}\n";
        let out = preprocess_bib(src);
        // The `@doi` must be normalised to `doi` so Typst parses
        // it as a field name.
        assert!(
            !out.contains("@doi"),
            "stray @doi should be stripped; got:\n{}",
            out
        );
        assert!(
            out.contains("doi = {10.1109/abc}"),
            "field should survive without @; got:\n{}",
            out
        );
    }

    #[test]
    fn passes_through_comments_and_preamble() {
        let src = "% this is a comment\n@preamble{ \"\\newcommand{\\foo}{bar}\" }\n@article{x, year=2024}\n";
        let out = preprocess_bib(src);
        assert!(out.contains("@preamble"), "preamble dropped: {}", out);
    }
}
