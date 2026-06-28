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

// Phase A is intentionally additive: the IR is built and unit-tested here but is
// not wired into the emitter until Phase B, so in non-test builds nothing yet
// consumes `lower`/`Node`. The allow keeps the build warning-free until then.
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

/// Build an owned [`Tree`] mirroring `ts_tree`. This Phase-A implementation is a
/// faithful 1:1 lowering — same kinds, spans, field names, and child order, with
/// ERROR/MISSING nodes preserved verbatim. Quirk normalization is deliberately
/// *not* done here yet (Phase C).
///
/// Lowering is iterative (an explicit work stack) rather than recursive so a
/// pathologically deep parse tree can't overflow the stack during this pass.
pub fn lower(ts_tree: &TsTree, _src: &str) -> Tree {
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

    Tree { nodes }
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

    #[test]
    fn mirrors_underscore_label_quirk() {
        // tree-sitter-latex truncates the label key at `_`; lowering must mirror
        // that (warts and all) in Phase A — the fix comes in Phase C.
        check_round_trip(r#"\section{Intro}\label{sec:edl_objective} text"#);
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
