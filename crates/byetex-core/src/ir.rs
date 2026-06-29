//! Owned lowering IR sitting between the tree-sitter parse tree and the emitter.
//!
//! This is the *anti-corruption layer* between `tree-sitter-latex` and ByeTex's
//! 17k-line emit machinery. Today the emitter walks `tree_sitter::Node` directly,
//! so every grammar quirk leaks straight into emit logic and is worked around
//! ad hoc in dozens of scattered places.
//!
//! The migration plan:
//!   * **Phase A (this module):** build a faithful, owned 1:1 mirror of the
//!     tree-sitter tree. Its [`Node`] handle exposes exactly the subset of the
//!     `tree_sitter::Node` API the emitter uses, so it is a drop-in type.
//!   * **Phase B:** flip the emitter from `tree_sitter::Node` onto [`Node`] by a
//!     mechanical type substitution — proven safe by byte-identical snapshots,
//!     because lowering is 1:1.
//!   * **Phase C:** move the grammar-quirk normalizers (underscore-truncated
//!     labels, sibling-attached args, ERROR-node recovery) *into* [`lower`], so
//!     they run once here instead of N times across `emit/`.
//!
//! Like `tree_sitter`, the tree owns all node storage in a flat arena and [`Node`]
//! is a cheap `Copy` handle into it. This keeps the same borrow shape the emitter
//! already relies on (and actually *removes* the `Tree`-must-outlive-`Node`
//! lifetime juggling, since the arena is a single owned value).

// `ir::Node` is a deliberately complete mirror of the subset of the
// `tree_sitter::Node` API the emitter relies on. A few accessors (`parent`,
// `is_named`) are part of that mirror but not yet exercised by the migrated emit
// path; they're kept for API parity and the later quirk-normalization phase, so
// the module allows dead code rather than dropping pieces of the surface.
#![allow(dead_code)]

use tree_sitter::Tree as TsTree;

/// A position in the source, mirroring `tree_sitter::Point`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub row: usize,
    pub column: usize,
}

/// Per-node storage held in the [`Tree`] arena.
#[derive(Debug)]
struct NodeData {
    kind: Box<str>,
    start_byte: usize,
    end_byte: usize,
    start_position: Point,
    end_position: Point,
    named: bool,
    /// Mirror of tree-sitter's `is_missing()` — a zero-width node the parser
    /// synthesized during error recovery (e.g. the `}` it inserts for an
    /// underscore-truncated label key). Kept so normalization can identify
    /// synthesized tokens precisely rather than guessing from a zero-width span.
    missing: bool,
    field_name: Option<Box<str>>,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// An owned mirror of a tree-sitter parse tree. Every node lives in a flat arena;
/// [`Node`] is a cheap copyable handle into it (mirroring `tree_sitter::Node`).
#[derive(Debug)]
pub struct Tree {
    nodes: Vec<NodeData>,
}

impl Tree {
    /// The root node. `lower` always produces at least the root, so the arena is
    /// never empty and index 0 is always the root.
    pub fn root_node(&self) -> Node<'_> {
        Node {
            tree: self,
            id: 0,
        }
    }
}

/// A cheap, copyable handle to one node in a [`Tree`], mirroring the subset of
/// `tree_sitter::Node`'s API that the emitter uses.
#[derive(Debug, Clone, Copy)]
pub struct Node<'a> {
    tree: &'a Tree,
    id: usize,
}

impl<'a> Node<'a> {
    #[inline]
    fn data(&self) -> &'a NodeData {
        &self.tree.nodes[self.id]
    }

    #[inline]
    fn handle(&self, id: usize) -> Node<'a> {
        Node {
            tree: self.tree,
            id,
        }
    }

    pub fn kind(&self) -> &'a str {
        &self.data().kind
    }

    pub fn start_byte(&self) -> usize {
        self.data().start_byte
    }

    pub fn end_byte(&self) -> usize {
        self.data().end_byte
    }

    pub fn start_position(&self) -> Point {
        self.data().start_position
    }

    pub fn end_position(&self) -> Point {
        self.data().end_position
    }

    /// Whether this node is a *named* node (as opposed to an anonymous token like
    /// `{` or `$`). Mirrors `tree_sitter::Node::is_named`.
    pub fn is_named(&self) -> bool {
        self.data().named
    }

    pub fn child_count(&self) -> usize {
        self.data().children.len()
    }

    pub fn child(&self, i: usize) -> Option<Node<'a>> {
        self.data().children.get(i).map(|&id| self.handle(id))
    }

    pub fn parent(&self) -> Option<Node<'a>> {
        self.data().parent.map(|id| self.handle(id))
    }

    /// Returns a no-op [`Cursor`] for API compatibility with
    /// `tree_sitter::Node::walk`. The cursor carries no state; iteration borrows
    /// directly from the arena.
    pub fn walk(&self) -> Cursor {
        Cursor
    }

    /// Iterate over all children. The `_cursor` argument is ignored — it exists
    /// only to keep the call signature identical to `tree_sitter::Node::children`
    /// so the emitter migration is a pure type substitution.
    pub fn children(&self, _cursor: &mut Cursor) -> Children<'a> {
        Children {
            tree: self.tree,
            ids: &self.data().children,
            pos: 0,
            named_only: false,
        }
    }

    /// Iterate over the *named* children only. Mirrors
    /// `tree_sitter::Node::named_children`.
    pub fn named_children(&self, _cursor: &mut Cursor) -> Children<'a> {
        Children {
            tree: self.tree,
            ids: &self.data().children,
            pos: 0,
            named_only: true,
        }
    }

    /// The first child carrying the grammar field `name`, if any. Mirrors
    /// `tree_sitter::Node::child_by_field_name`.
    pub fn child_by_field_name(&self, name: &str) -> Option<Node<'a>> {
        self.data().children.iter().find_map(|&id| {
            if self.tree.nodes[id].field_name.as_deref() == Some(name) {
                Some(self.handle(id))
            } else {
                None
            }
        })
    }
}

/// No-op stand-in for `tree_sitter::TreeCursor`, returned by [`Node::walk`]. The
/// real cursor threads mutable state through `children()`; ours holds none, since
/// the arena is directly indexable.
pub struct Cursor;

/// Iterator over a node's children, mirroring the iterator returned by
/// `tree_sitter::Node::children` / `named_children`.
pub struct Children<'a> {
    tree: &'a Tree,
    ids: &'a [usize],
    pos: usize,
    named_only: bool,
}

impl<'a> Iterator for Children<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Node<'a>> {
        while self.pos < self.ids.len() {
            let id = self.ids[self.pos];
            self.pos += 1;
            if !self.named_only || self.tree.nodes[id].named {
                return Some(Node {
                    tree: self.tree,
                    id,
                });
            }
        }
        None
    }
}

/// Build an owned [`Tree`] from `ts_tree`. The bulk is a faithful 1:1 lowering —
/// same kinds, spans, field names, and child order — followed by targeted
/// grammar-quirk normalization passes (Phase C) that the emitter would otherwise
/// have to work around. Currently: [`normalize_truncated_labels`] repairs
/// underscore-truncated label keys.
///
/// Lowering is iterative (an explicit work stack) rather than recursive so a
/// pathologically deep parse tree can't overflow the stack during this pass.
pub fn lower(ts_tree: &TsTree, src: &str) -> Tree {
    let mut nodes: Vec<NodeData> = Vec::new();

    // Work items: (tree-sitter node, parent arena id, field name on the edge).
    let mut stack: Vec<(tree_sitter::Node<'_>, Option<usize>, Option<String>)> =
        vec![(ts_tree.root_node(), None, None)];

    while let Some((tsn, parent, field)) = stack.pop() {
        let id = nodes.len();
        let sp = tsn.start_position();
        let ep = tsn.end_position();
        nodes.push(NodeData {
            kind: tsn.kind().into(),
            start_byte: tsn.start_byte(),
            end_byte: tsn.end_byte(),
            start_position: Point {
                row: sp.row,
                column: sp.column,
            },
            end_position: Point {
                row: ep.row,
                column: ep.column,
            },
            named: tsn.is_named(),
            missing: tsn.is_missing(),
            field_name: field.map(String::into_boxed_str),
            parent,
            children: Vec::new(),
        });
        if let Some(p) = parent {
            nodes[p].children.push(id);
        }

        // Collect this node's children (with their field names) in source order,
        // then push them onto the stack in REVERSE so they pop — and are appended
        // to `children` — in forward order.
        let mut cursor = tsn.walk();
        let mut kids: Vec<(tree_sitter::Node<'_>, Option<String>)> = Vec::new();
        if cursor.goto_first_child() {
            loop {
                let field_name = cursor.field_name().map(str::to_string);
                kids.push((cursor.node(), field_name));
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        for (child, field_name) in kids.into_iter().rev() {
            stack.push((child, Some(id), field_name));
        }
    }

    // Repair tree-sitter-latex's underscore-truncated label keys before the tree
    // is frozen (Phase C1).
    normalize_truncated_labels(&mut nodes, src);

    Tree { nodes }
}

/// Index one past the `}` that closes the label-key brace at `open`.
///
/// A LaTeX label key is flat (no nested groups) and single-line, so this returns
/// the first unescaped `}` and BAILS (`None`) the moment it meets a nested `{` or
/// a newline. That bound is what makes promoting the recovery into a
/// tree-mutating prune safe: without it, a label whose own `}` is missing would
/// balance against an unrelated later `}` and the prune would delete the
/// intervening real document content (review #2). `\X` escapes are skipped with a
/// bounds guard so a key ending in a lone backslash can't run the index past the
/// buffer (review #4).
fn label_brace_end(bytes: &[u8], open: usize) -> Option<usize> {
    if bytes.get(open) != Some(&b'{') {
        return None;
    }
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'}' => return Some(i + 1),
            // Nested group or line break ⇒ this isn't a flat single-line key; the
            // real close is missing, so don't risk engulfing real content.
            b'{' | b'\n' | b'\r' => return None,
            b'\\' => i += 2, // skip the escaped byte (bounds-safe: i may pass len)
            _ => i += 1,
        }
    }
    None
}

/// Precompute the byte offset of the start of each line (index 0 is line 0 at
/// byte 0). Lets [`byte_to_point`] resolve a position in O(log lines) instead of
/// re-scanning the source from byte 0 on every call (review #6).
fn line_start_offsets(src: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, &b) in src.as_bytes().iter().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Byte offset → `Point` via a precomputed line-start table. Columns are
/// byte-based to match tree-sitter's own `Point` convention.
fn byte_to_point(line_starts: &[usize], byte: usize) -> Point {
    // Row = index of the last line start ≤ byte. `partition_point` returns the
    // count of starts ≤ byte; subtract 1 (line_starts[0] == 0 guarantees ≥ 1).
    let row = line_starts.partition_point(|&ls| ls <= byte).saturating_sub(1);
    Point {
        row,
        column: byte.saturating_sub(line_starts[row]),
    }
}

/// Repair tree-sitter-latex's underscore-truncated label keys.
///
/// The grammar stops a `label`/`label_reference` key at the first `_`, so
/// `\label{eq:edl_objective}` lowers to a `curly_group_label` ending at `edl`
/// with a synthesized `MISSING "}"`, and the tail (`_objective`) plus an orphan
/// `}` leak out as following `subscript`/`word`/`ERROR` siblings (sometimes
/// across node levels — e.g. inside a parent `text` for `\ref`).
///
/// This pass moves the *label-key* recovery that previously lived scattered
/// across the emit layer (a source byte-scan in `extract_label_name_and_end` /
/// `extract_label_ref_keys_and_end` and its `skip_until`) into the single
/// chokepoint here: it extends the label group (and its truncated `label` leaf
/// and any clipped ancestors) to the real closing brace, drops the synthesized
/// `MISSING` brace, and prunes every node whose span falls entirely inside the
/// leaked region. After this, the emit layer reads the label key straight off the
/// node span with no special-casing.
///
/// It only fixes *flat, single-line* label keys (see [`label_brace_end`]); a
/// malformed key whose close is missing is left untouched so the prune can never
/// engulf real content. Stray single-brace `ERROR` nodes from sources OTHER than
/// underscore-truncation (e.g. an unbalanced `}` in the body) are NOT this pass's
/// concern — they're still dropped by the dedicated arm in `emit_node`.
fn normalize_truncated_labels(nodes: &mut [NodeData], src: &str) {
    let bytes = src.as_bytes();
    let n = nodes.len();

    // Pass 1 — detect truncated label groups: (group_id, old_end, real_close).
    let mut fixes: Vec<(usize, usize, usize)> = Vec::new();
    for id in 0..n {
        match &*nodes[id].kind {
            "curly_group_label" | "curly_group_label_list" => {}
            _ => continue,
        }
        let open = nodes[id].start_byte;
        let Some(real_close) = label_brace_end(bytes, open) else {
            continue;
        };
        let old_end = nodes[id].end_byte;
        if real_close > old_end {
            fixes.push((id, old_end, real_close));
        }
    }
    if fixes.is_empty() {
        return;
    }
    let line_starts = line_start_offsets(src);

    // Pass 2 — extend the label group, its truncated `label` leaf, and any
    // ancestors clipped at the truncation point; drop the synthesized MISSING `}`.
    for &(id, old_end, real_close) in &fixes {
        nodes[id].end_byte = real_close;
        nodes[id].end_position = byte_to_point(&line_starts, real_close);

        let kids = nodes[id].children.clone();
        let mut kept = Vec::with_capacity(kids.len());
        for k in kids {
            // Drop the parser-synthesized MISSING close brace (review #11: use the
            // mirrored `missing` bit, not a zero-width-span guess).
            if nodes[k].missing {
                continue;
            }
            // Extend ONLY the `label` leaf that was clipped at the truncation
            // point — a label_list may keep an earlier, un-truncated leaf whose
            // span must not be stretched over the whole key (review #5).
            if &*nodes[k].kind == "label" && nodes[k].end_byte == old_end {
                nodes[k].end_byte = real_close - 1;
                nodes[k].end_position = byte_to_point(&line_starts, real_close - 1);
            }
            kept.push(k);
        }
        nodes[id].children = kept;

        // Extend ancestors that were clipped at the truncation point. By tree
        // containment an ancestor's end is always ≥ the group's end (old_end), so
        // any ancestor ending strictly inside the leaked region [old_end,
        // real_close) is there only because it held the leaked tail; stop at the
        // first ancestor that already reaches the real close.
        let mut parent = nodes[id].parent;
        while let Some(pid) = parent {
            if nodes[pid].end_byte >= real_close {
                break;
            }
            nodes[pid].end_byte = real_close;
            nodes[pid].end_position = byte_to_point(&line_starts, real_close);
            parent = nodes[pid].parent;
        }
    }

    // Pass 3 — prune every node whose span is fully inside a leaked region
    // (the tail words/subscripts and the orphan ERROR brace). The label
    // groups/leaves extended above start before `old_end`, so they're never
    // pruned. Nodes stay in the arena but are unlinked from their parents, so
    // the walk no longer reaches them.
    let mut pruned = vec![false; n];
    for (id, flag) in pruned.iter_mut().enumerate() {
        let (s, e) = (nodes[id].start_byte, nodes[id].end_byte);
        if fixes
            .iter()
            .any(|&(_, old_end, real_close)| s >= old_end && s < real_close && e <= real_close)
        {
            *flag = true;
        }
    }
    for id in 0..n {
        if nodes[id].children.iter().any(|&c| pruned[c]) {
            let kept: Vec<usize> = nodes[id]
                .children
                .iter()
                .copied()
                .filter(|&c| !pruned[c])
                .collect();
            nodes[id].children = kept;
        }
    }
}

/// Parse `src` with the tree-sitter grammar and immediately lower it to the owned
/// IR. This is the single entry point the emitter uses instead of
/// `parser::parse` — the returned [`Tree`] owns its arena (it does not borrow the
/// transient tree-sitter tree), so `tree.root_node()` walks exactly as before.
pub fn parse_and_lower(src: &str) -> Tree {
    let ts_tree = crate::parser::parse(src);
    lower(&ts_tree, src)
}

/// Sentinel byte that temporarily stands in for `_` inside cross-reference keys
/// during parsing. It is a control byte that never appears in real LaTeX source,
/// is treated by tree-sitter-latex as an ordinary key-word character (so it does
/// NOT trip the math-subscript misparse), survives label sanitization unchanged
/// (see `is_typst_label_char`), and is restored to `_` in the final Typst output.
pub(crate) const REFKEY_US_SENTINEL: char = '\u{1f}';

/// Neutralize `_` inside cross-reference / label command keys *before* the
/// tree-sitter parse.
///
/// tree-sitter-latex mis-reads an `_` in a `\label{a_b}` / `\eqref{a_b}` key as a
/// math subscript; on a complex document the accumulated mis-parses prevent the
/// `document` environment from forming at all (the parse root becomes one giant
/// ERROR node and the emitter then raw-copies the un-recognised gaps — leaking
/// `\begin{document}` and dropping section headings, corpus 2605.22728:
/// 1→8 recovered headings once these keys are neutralised). We replace each `_`
/// with [`REFKEY_US_SENTINEL`] (a SAME-LENGTH 1-byte substitution, so every byte
/// offset the emitter relies on is preserved) and restore it on emit.
///
/// Scope is deliberately narrow: only the brace key of the listed reference /
/// label commands. Math subscripts (`x_1`) and `\cite` keys (matched against the
/// bibliography) are left untouched.
pub(crate) fn neutralize_ref_key_underscores(src: &str) -> String {
    // Longer command names first so a prefix (`\label`, `\ref`, `\cref`) never
    // shadows its extension (`\labelcref`, `\refrange`, `\crefrange`); the
    // trailing non-alphabetic guard is the real disambiguator, this is belt-and-
    // suspenders. `\cite*` is intentionally absent (bib keys stay verbatim).
    const CMDS: &[&str] = &[
        "\\labelcref",
        "\\namecref",
        "\\nameref",
        "\\cpageref",
        "\\crefrange",
        "\\Crefrange",
        "\\autoref",
        "\\pageref",
        "\\eqref",
        "\\vref",
        "\\cref",
        "\\Cref",
        "\\label",
        "\\ref",
    ];
    if !src.contains('_') {
        return src.to_string();
    }
    let bytes = src.as_bytes();
    let mut out = src.as_bytes().to_vec();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }
        let rest = &src[i..];
        let cmd = CMDS.iter().find(|c| {
            rest.starts_with(**c)
                && !rest[c.len()..].starts_with(|ch: char| ch.is_ascii_alphabetic())
        });
        let Some(cmd) = cmd else {
            i += 1;
            continue;
        };
        let mut j = i + cmd.len();
        // Skip whitespace, then any optional `[...]` argument(s).
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        while j < bytes.len() && bytes[j] == b'[' {
            let mut depth = 0i32;
            while j < bytes.len() {
                match bytes[j] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            j += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
        }
        // The mandatory `{key}` — replace `_` (0x5f) with the sentinel byte.
        if j < bytes.len() && bytes[j] == b'{' {
            j += 1;
            let mut depth = 1i32;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b'_' => out[j] = REFKEY_US_SENTINEL as u8,
                    _ => {}
                }
                j += 1;
            }
        }
        i = j;
    }
    // SAFETY: only 0x5f ('_') bytes were replaced with 0x1f, both standalone ASCII
    // bytes, so the buffer remains valid UTF-8.
    String::from_utf8(out).expect("same-length ASCII byte substitution preserves UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    /// Recursively assert that an `ir::Node` reports the same structure as the
    /// tree-sitter node it was lowered from: kind, byte span, position, child
    /// count, and — recursively — every child in order.
    fn assert_mirrors(ts: tree_sitter::Node<'_>, ir: Node<'_>) {
        assert_eq!(ts.kind(), ir.kind(), "kind mismatch");
        assert_eq!(ts.start_byte(), ir.start_byte(), "start_byte mismatch");
        assert_eq!(ts.end_byte(), ir.end_byte(), "end_byte mismatch");
        assert_eq!(
            ts.start_position().row,
            ir.start_position().row,
            "start row mismatch"
        );
        assert_eq!(
            ts.start_position().column,
            ir.start_position().column,
            "start col mismatch"
        );
        assert_eq!(ts.is_named(), ir.is_named(), "is_named mismatch");
        assert_eq!(
            ts.child_count(),
            ir.child_count(),
            "child_count mismatch for kind {}",
            ts.kind()
        );

        let mut tc = ts.walk();
        let ts_children: Vec<_> = ts.children(&mut tc).collect();
        let mut ic = ir.walk();
        let ir_children: Vec<_> = ir.children(&mut ic).collect();
        assert_eq!(ts_children.len(), ir_children.len(), "children len mismatch");
        for (t, i) in ts_children.iter().zip(ir_children.iter()) {
            assert_mirrors(*t, *i);
        }
    }

    fn check_round_trip(src: &str) {
        let ts_tree = parser::parse(src);
        let ir_tree = lower(&ts_tree, src);
        assert_mirrors(ts_tree.root_node(), ir_tree.root_node());
    }

    #[test]
    fn mirrors_simple_document() {
        check_round_trip(
            r#"\documentclass{article}
\begin{document}
Hello \textbf{world}.
\end{document}
"#,
        );
    }

    /// Collect `(kind, start_byte, end_byte)` for every node in the IR, pre-order.
    fn flatten(node: Node<'_>, out: &mut Vec<(String, usize, usize)>) {
        out.push((node.kind().to_string(), node.start_byte(), node.end_byte()));
        let mut c = node.walk();
        for child in node.children(&mut c) {
            flatten(child, out);
        }
    }

    #[test]
    fn lower_normalizes_truncated_label_definition() {
        // tree-sitter-latex truncates the label key at the first `_`, leaking the
        // tail (`_objective`) as `subscript`/`word` siblings plus an orphan `}` as
        // an ERROR node. Phase C1: `lower` repairs this — the curly_group_label
        // spans the whole `{...}`, the leaked nodes are pruned, and no
        // MISSING/ERROR survives.
        let src = r#"\section{Intro}\label{sec:edl_objective} text"#;
        let ts_tree = parser::parse(src);
        let ir = lower(&ts_tree, src);

        let mut nodes = Vec::new();
        flatten(ir.root_node(), &mut nodes);

        let label_groups: Vec<_> = nodes
            .iter()
            .filter(|(k, _, _)| k == "curly_group_label")
            .collect();
        assert_eq!(label_groups.len(), 1, "exactly one curly_group_label");
        let (_, s, e) = label_groups[0];
        assert_eq!(
            &src[*s..*e],
            "{sec:edl_objective}",
            "label group must span the full braces"
        );

        assert!(
            nodes
                .iter()
                .any(|(k, a, b)| k == "label" && &src[*a..*b] == "sec:edl_objective"),
            "the full label key is recoverable from the (label) leaf"
        );
        assert!(
            !nodes.iter().any(|(k, _, _)| k == "ERROR"),
            "no ERROR node should survive normalization"
        );
        assert!(
            !nodes.iter().any(|(k, a, b)| k == "}" && a == b),
            "no synthesized MISSING brace should survive"
        );
        assert!(
            !nodes.iter().any(|(k, _, _)| k == "subscript"),
            "the leaked `_objective` subscript must be pruned"
        );
        assert!(
            nodes.iter().any(|(k, a, b)| k == "word" && &src[*a..*b] == "text"),
            "trailing real content ('text') is preserved"
        );
    }

    #[test]
    fn lower_normalizes_truncated_label_reference() {
        // Same quirk for `\ref{...}` — here the leak even crosses node levels
        // (siblings inside a `text` node plus a top-level ERROR brace).
        let src = r#"see \ref{thm:UAP_general_dim} now"#;
        let ts_tree = parser::parse(src);
        let ir = lower(&ts_tree, src);

        let mut nodes = Vec::new();
        flatten(ir.root_node(), &mut nodes);

        let groups: Vec<_> = nodes
            .iter()
            .filter(|(k, _, _)| k == "curly_group_label_list")
            .collect();
        assert_eq!(groups.len(), 1, "exactly one curly_group_label_list");
        let (_, s, e) = groups[0];
        assert_eq!(
            &src[*s..*e],
            "{thm:UAP_general_dim}",
            "ref label group must span the full braces"
        );
        assert!(
            !nodes.iter().any(|(k, _, _)| k == "ERROR"),
            "no ERROR node should survive"
        );
        assert!(
            !nodes.iter().any(|(k, _, _)| k == "subscript"),
            "leaked subscript nodes from the ref tail must be pruned"
        );
        assert!(
            nodes.iter().any(|(k, a, b)| k == "word" && &src[*a..*b] == "now"),
            "trailing real content ('now') is preserved"
        );
    }

    #[test]
    fn lower_leaves_well_formed_labels_untouched() {
        // A label with no underscore parses cleanly; normalization must be a no-op.
        let src = r#"\label{sec:intro} body"#;
        check_round_trip(src);
    }

    #[test]
    fn lower_does_not_engulf_content_on_unbalanced_label() {
        // Review #2: a truncated label whose own `}` is missing must NOT balance
        // against a later group's `}` and prune the intervening real content.
        // Here `\ref{fig:a_b \mbox{x}}` has the label brace effectively unclosed
        // (a nested `{` appears first), so label_brace_end bails and the fix is
        // abandoned — leaving `KEEPME` and `x` intact in the tree.
        let src = r#"see \ref{fig:a_b \mbox{x}} KEEPME after"#;
        let ir = lower(&parser::parse(src), src);
        let mut nodes = Vec::new();
        flatten(ir.root_node(), &mut nodes);
        assert!(
            nodes.iter().any(|(k, a, b)| k == "word" && &src[*a..*b] == "KEEPME"),
            "real content after an unbalanced label must not be pruned"
        );
        assert!(
            nodes.iter().any(|(_, a, b)| &src[*a..*b] == "x"),
            "content nested after the unclosed label brace must survive"
        );
    }

    #[test]
    fn lower_handles_label_key_with_trailing_backslash() {
        // Review #4: a key whose escape (`\`) is the last scanned byte must not
        // run the index past the buffer; lowering must complete without panic.
        for src in [r#"\ref{a_b\"#, r#"\label{x_y\}"#, r#"text \ref{a_\"#] {
            let _ = lower(&parser::parse(src), src); // must not panic
        }
    }

    #[test]
    fn byte_to_point_resolves_rows_and_columns() {
        // Review #6: byte_to_point via the precomputed line-start table.
        let src = "ab\ncde\n\nxy";
        let ls = line_start_offsets(src);
        assert_eq!(byte_to_point(&ls, 0), Point { row: 0, column: 0 }); // 'a'
        assert_eq!(byte_to_point(&ls, 1), Point { row: 0, column: 1 }); // 'b'
        assert_eq!(byte_to_point(&ls, 3), Point { row: 1, column: 0 }); // 'c'
        assert_eq!(byte_to_point(&ls, 5), Point { row: 1, column: 2 }); // 'e'
        assert_eq!(byte_to_point(&ls, 8), Point { row: 3, column: 0 }); // 'x' (after blank line)
    }

    #[test]
    fn mirrors_optional_and_required_args() {
        check_round_trip(r#"\xrightarrow[below]{above} and \notempty[X]{Y}"#);
    }

    #[test]
    fn mirrors_math_environment() {
        check_round_trip(
            r#"\begin{align}
  a_1 &= b + c \\
  x &= \frac{1}{2}
\end{align}"#,
        );
    }

    #[test]
    fn mirrors_nested_groups() {
        check_round_trip(r#"{\bf bold {\it italic} again} \verb|raw_text|"#);
    }

    #[test]
    fn mirrors_empty_input() {
        check_round_trip("");
    }

    #[test]
    fn named_children_filters_anonymous_tokens() {
        let src = r#"\textbf{hi}"#;
        let ts_tree = parser::parse(src);
        let ir_tree = lower(&ts_tree, src);

        // Walk to the curly_group and compare named vs all child counts between
        // tree-sitter and the IR at the same node, confirming the named filter
        // matches tree-sitter's own notion of "named".
        fn compare_named(ts: tree_sitter::Node<'_>, ir: Node<'_>) {
            let mut tc = ts.walk();
            let ts_named = ts.named_children(&mut tc).count();
            let mut ic = ir.walk();
            let ir_named = ir.named_children(&mut ic).count();
            assert_eq!(ts_named, ir_named, "named child count mismatch at {}", ts.kind());

            let mut tc2 = ts.walk();
            let ts_kids: Vec<_> = ts.children(&mut tc2).collect();
            let mut ic2 = ir.walk();
            let ir_kids: Vec<_> = ir.children(&mut ic2).collect();
            for (t, i) in ts_kids.iter().zip(ir_kids.iter()) {
                compare_named(*t, *i);
            }
        }
        compare_named(ts_tree.root_node(), ir_tree.root_node());
    }

    #[test]
    fn field_names_are_preserved() {
        // Pick a construct whose grammar uses a named field. `\begin{...}` env
        // names and includes carry field names in tree-sitter-latex; verify any
        // field name tree-sitter assigns is reproduced by `child_by_field_name`.
        let src = r#"\includegraphics[width=2cm]{fig.png}"#;
        let ts_tree = parser::parse(src);
        let ir_tree = lower(&ts_tree, src);

        fn compare_fields(ts: tree_sitter::Node<'_>, ir: Node<'_>) {
            let mut tc = ts.walk();
            let mut cursor = ts.walk();
            if cursor.goto_first_child() {
                loop {
                    if let Some(field) = cursor.field_name() {
                        let ts_child = cursor.node();
                        let ir_child = ir
                            .child_by_field_name(field)
                            .expect("ir should expose the same field");
                        assert_eq!(ts_child.kind(), ir_child.kind());
                        assert_eq!(ts_child.start_byte(), ir_child.start_byte());
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            let ts_kids: Vec<_> = ts.children(&mut tc).collect();
            let mut ic = ir.walk();
            let ir_kids: Vec<_> = ir.children(&mut ic).collect();
            for (t, i) in ts_kids.iter().zip(ir_kids.iter()) {
                compare_fields(*t, *i);
            }
        }
        compare_fields(ts_tree.root_node(), ir_tree.root_node());
    }
}
