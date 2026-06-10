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
