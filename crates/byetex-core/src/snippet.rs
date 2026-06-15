//! Snippet utilities for agents working on reproducible, in-the-small tasks:
//! [`explain`] dumps the per-node LaTeX→Typst mapping ("why did this LaTeX emit
//! this Typst?"), and [`convert_fragment`] converts a bare LaTeX fragment with a
//! context hint so math fragments land in Typst math mode rather than being
//! mistaken for unknown text commands.

use serde::Serialize;

use crate::{convert, convert_capturing_source_map, ConvertOptions, ConvertOutput};

/// One node's provenance: the LaTeX fragment and the Typst it produced.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Explanation {
    /// The LaTeX source fragment this node came from.
    pub src_fragment: String,
    /// The Typst text this fragment produced.
    pub typst_output: String,
    /// Byte offsets of the fragment in the input source (`src_start..src_end`).
    pub src_start: usize,
    pub src_end: usize,
}

/// Convert `source` capturing the content-anchored source map and return a
/// per-node LaTeX→Typst mapping. Nodes whose fragment or output is empty /
/// whitespace are dropped (they carry no signal). Useful for reproducible
/// debugging: an agent can see exactly which fragment produced which Typst.
pub fn explain(source: &str, opts: &ConvertOptions) -> Vec<Explanation> {
    let out = convert_capturing_source_map(source, opts);
    // Nested nodes (a text node and the parent that re-emits it) can record the
    // same span → output twice; collapse those identical rows for a clean map.
    let mut seen = std::collections::HashSet::new();
    out.source_map
        .iter()
        .filter_map(|n| {
            let (a, b) = n.src;
            // Guard against degenerate / out-of-range spans before slicing.
            if a >= b || b > source.len() || !source.is_char_boundary(a) || !source.is_char_boundary(b)
            {
                return None;
            }
            let frag = &source[a..b];
            if frag.trim().is_empty() || n.output.trim().is_empty() {
                return None;
            }
            Some(Explanation {
                src_fragment: frag.to_string(),
                typst_output: n.output.clone(),
                src_start: a,
                src_end: b,
            })
        })
        .filter(|e| seen.insert((e.src_start, e.src_end, e.typst_output.clone())))
        .collect()
}

/// The mode a bare LaTeX fragment is in, so it converts into the right Typst
/// context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentContext {
    /// Inline text (the default).
    Inline,
    /// Block / paragraph text. Converts the same as `Inline` today.
    Block,
    /// Inline math — the fragment is wrapped in `$…$` before conversion.
    Math,
    /// Display math — the fragment is wrapped in `\[…\]` before conversion.
    MathDisplay,
}

impl FragmentContext {
    /// Parse an MCP `context_hint` string (`inline | block | math |
    /// math_display`). Unknown or empty hints default to [`Self::Inline`].
    pub fn parse(hint: &str) -> Self {
        match hint.trim() {
            "math" => FragmentContext::Math,
            "math_display" => FragmentContext::MathDisplay,
            "block" => FragmentContext::Block,
            _ => FragmentContext::Inline,
        }
    }
}

/// Convert a bare LaTeX `fragment` in the given `ctx`. Math contexts wrap the
/// fragment in `$…$` / `\[…\]` first so math commands (e.g. `\frac`) convert
/// correctly instead of warning as unknown text commands; inline/block convert
/// the fragment as-is.
pub fn convert_fragment(
    fragment: &str,
    ctx: FragmentContext,
    opts: &ConvertOptions,
) -> ConvertOutput {
    let wrapped = match ctx {
        FragmentContext::Math => format!("${fragment}$"),
        FragmentContext::MathDisplay => format!("\\[{fragment}\\]"),
        FragmentContext::Inline | FragmentContext::Block => fragment.to_string(),
    };
    convert(&wrapped, opts)
}
