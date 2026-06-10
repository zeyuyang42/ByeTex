//! Content-anchored source map. Each emitted node records the `.typ` text it
//! produced and its originating LaTeX source byte-range. A typst compile error
//! (a line in the final `.typ`) is resolved to a source span by matching the
//! line's TEXT against the node that produced it — robust to the byte shifts
//! that `finish()` / `post_process_typography` introduce after emission.

/// One emitted node's provenance: the source it came from and the text it wrote.
#[derive(Debug, Clone)]
pub struct NodeOutput {
    /// Byte range in the LaTeX source.
    pub src: (usize, usize),
    /// The `.typ` text this node produced (pre-post-process).
    pub output: String,
}

fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Resolve a `.typ` error line to its originating LaTeX source span.
///
/// **Pass 1 — full containment:** Among nodes whose (normalized) output
/// *contains* the (normalized) error line, return the `src` of the one with
/// the **shortest** output (most specific). This handles the common case where
/// a parent node re-emits everything a child node produced.
///
/// **Pass 2 — reverse containment:** Among nodes whose (normalized) output is
/// *contained in* the (normalized) error line (i.e., the node output is a
/// substring of the line), return the `src` of the one with the shortest
/// output. This handles post-processing that wraps a node's text in extra
/// punctuation (e.g., `#hide[$arrival$]` → `(#hide[$arrival$])`).
///
/// Returns `None` if neither pass finds a match.
pub fn resolve_error_line(map: &[NodeOutput], typ_line: &str) -> Option<(usize, usize)> {
    let needle = normalize(typ_line);
    if needle.is_empty() {
        return None;
    }

    // Pass 1: nodes whose output contains the whole (normalized) line.
    // Shortest output wins (most specific / least context).
    let full = map
        .iter()
        .filter(|n| normalize(&n.output).contains(&needle))
        .min_by_key(|n| n.output.len());
    if let Some(n) = full {
        return Some(n.src);
    }

    // Pass 2: reverse containment — nodes whose (normalized) output is a
    // substring of the (normalized) error line. Handles wrapping introduced by
    // post-processing (e.g., surrounding parens, punctuation).
    // Shortest output wins; require at least 3 chars to avoid noisy matches.
    map.iter()
        .filter(|n| {
            let out = normalize(&n.output);
            out.len() >= 3 && needle.contains(&out)
        })
        .min_by_key(|n| n.output.len())
        .map(|n| n.src)
}

/// The maximal run of non-whitespace characters in `line` that touches column
/// `col` (0-based, char index, as typst reports). Used to anchor an error on
/// the exact token it points at rather than the whole line. Returns `None` for
/// an empty line or when `col` lands on whitespace with no adjacent token.
fn token_at(line: &str, col: usize) -> Option<&str> {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    if chars.is_empty() {
        return None;
    }
    // Clamp col into range; if it lands on whitespace, step left to the end of
    // the preceding token (typst often points just past the offending token).
    let mut idx = col.min(chars.len() - 1);
    while chars[idx].1.is_whitespace() {
        if idx == 0 {
            return None;
        }
        idx -= 1;
    }
    let mut start = idx;
    while start > 0 && !chars[start - 1].1.is_whitespace() {
        start -= 1;
    }
    let mut end = idx;
    while end + 1 < chars.len() && !chars[end + 1].1.is_whitespace() {
        end += 1;
    }
    let byte_start = chars[start].0;
    let byte_end = chars[end].0 + chars[end].1.len_utf8();
    Some(&line[byte_start..byte_end])
}

/// Resolve a `.typ` error to its source span using the error's COLUMN for
/// precision. A typst error line is often a whole paragraph (so a full-line
/// match only finds a coarse container), but its `col` points at the exact
/// failing token (e.g. `@comaskey:2022` in a multi-cite line). Resolve that
/// token first — the smallest node whose output contains it is the specific
/// construct — and fall back to [`resolve_error_line`] when the token is too
/// short or doesn't match.
pub fn resolve_error_at_col(
    map: &[NodeOutput],
    typ_line: &str,
    col: usize,
) -> Option<(usize, usize)> {
    if let Some(tok) = token_at(typ_line, col) {
        if tok.len() >= 3 {
            if let Some(span) = map
                .iter()
                .filter(|n| n.output.contains(tok))
                .min_by_key(|n| n.output.len())
                .map(|n| n.src)
            {
                return Some(span);
            }
        }
    }
    resolve_error_line(map, typ_line)
}
