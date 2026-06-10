# Multi-caption Float Splitting (subpar.grid) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a LaTeX float holds multiple captioned sub-blocks, emit a `#subpar.grid(...)` with one inner `figure(...)` per sub-block (own caption + own label + real sub-numbering), instead of dropping every caption after the first.

**Architecture:** All work is in `crates/byetex-core/src/emit/figures.rs` (`emit_figure`), plus a one-field flag in `crates/byetex-core/src/emit.rs` for a conditional `#import "@preview/subpar:0.2.2"`. We (1) refactor the existing single-figure assembly into a reusable `emit_figure_inner` string-builder, (2) add pure column-packing helpers, (3) handle explicit `subfigure`/`subtable` envs (Pattern A) and (4) top-level multi-`\captionof` (Pattern B) by emitting a `subpar.grid`. Single-caption floats stay byte-identical.

**Tech Stack:** Rust, tree-sitter-latex AST, `cargo test`, `cargo insta` snapshots, typst 0.14.2, `@preview/subpar:0.2.2`.

**Spec:** `docs/superpowers/specs/2026-06-10-multicaption-subpar-grid-design.md`

**Branch / worktree:** `feat/multicaption-subpar-grid` in worktree `/Users/zeyuyang42/Workspace/tools/ByeTex-labelfix` (already created off `origin/main` @ 8c341a5). The corpus is symlinked at `corpus/`. Build the worktree binary with `cargo build --release -p byetex-cli`; run the acceptance gate with `BYETEX_BIN="$(pwd)/target/release/byetex" bash scripts/acceptance.sh`.

**Conventions in this codebase:**
- Bare-fragment conversion for tests: `byetex_core::convert(src, &Default::default()).typst` returns the Typst string.
- Helpers already available in `emit/node_utils.rs`: `first_curly_group(node)`, `nth_curly_group(node, n)`, `environment_name(env, src) -> Option<String>`, `extract_label_name(node, src) -> Option<String>`. In `emit/escape.rs`: `sanitize_label_key(&str) -> String`. On `Emitter`: `render_curly_group_content(group) -> String`, `with_sub_buffer(|e| {...}) -> String`, `pick_label_to_attach(&[String]) -> Option<String>`, `render_subfigure_panel(node) -> Option<String>`.
- `emit_figure` is at `figures.rs:282`; its single-figure assembly tail is lines `386–440` (shown verbatim in Task 1).

---

## Task 1: Extract `emit_figure_inner` (byte-identical refactor)

Pull the body+kind+caption assembly into a helper that returns a `figure(...)` string (no leading `#`, no trailing label). Rewire the single-caption path through it. No behavior change — existing figure tests + snapshots are the regression guard.

**Files:**
- Modify: `crates/byetex-core/src/emit/figures.rs:386-424` (the assembly block)

- [ ] **Step 1: Run the existing figure tests to capture the green baseline**

Run: `cargo test -p byetex-core --test captionof --test multi_label_figure 2>&1 | tail -15`
Expected: all PASS (this is the baseline we must keep byte-identical).

- [ ] **Step 2: Add the `emit_figure_inner` helper**

Add this method to the `impl` block in `figures.rs` (place it just above `fn emit_figure`):

```rust
/// Render one captioned block as a Typst `figure(...)` string — no leading
/// `#`, no trailing `<label>`. Used both for the single-figure path and for
/// each panel of a `subpar.grid`. `kind` is `Some("table")` / `Some("image")`
/// or `None` (image default); `caption_text` is the already-rendered caption
/// body (without brackets) or `None`.
fn emit_figure_inner(
    &self,
    body_str: &str,
    kind: Option<&str>,
    caption_text: Option<&str>,
) -> String {
    let mut s = String::new();
    s.push_str("figure(\n  ");
    s.push_str(body_str);
    if let Some(k) = kind {
        let _ = write!(s, ",\n  kind: {}", k);
    }
    if let Some(text) = caption_text {
        let _ = write!(s, ",\n  caption: [{}]", text);
    }
    s.push_str(",\n)");
    s
}
```

Note: `write!` into a `String` needs `use std::fmt::Write as _;` — it is already imported at the top of `figures.rs` (the file already uses `write!(self.out, …)`). If the compiler complains about a missing trait import, add `use std::fmt::Write as _;` near the other `use` lines.

- [ ] **Step 3: Rewire the single-caption assembly to use the helper**

Replace lines `386–424` (from `self.ensure_paragraph_break();` through `self.out.push_str(",\n)");`) with:

```rust
        // Resolve kind: an explicit `\captionof{type}` wins; else a tabular
        // body implies `kind: table`; else image default.
        let mut kind: Option<&str> = None;
        if caption.is_none() {
            if let Some(c) = captionof {
                if let Some(type_arg) = nth_curly_group(c, 0) {
                    let ty = self.render_curly_group_content(type_arg);
                    kind = match ty.trim() {
                        "table" => Some("table"),
                        "figure" => Some("image"),
                        _ => None,
                    };
                }
            }
        }
        if kind.is_none() && body_is_table {
            kind = Some("table");
        }
        // Resolve caption text: `\caption{cap}` → 1st group; `\captionof{t}{cap}` → 2nd.
        let caption_node = caption.or(captionof);
        let caption_text = caption_node.and_then(|c| {
            let arg = if c.kind() == "generic_command" {
                nth_curly_group(c, 1)
            } else {
                first_curly_group(c)
            };
            arg.map(|a| self.render_curly_group_content(a))
        });
        let inner = self.emit_figure_inner(&body_str, kind, caption_text.as_deref());
        self.ensure_paragraph_break();
        self.out.push('#');
        self.out.push_str(&inner);
```

The label-attach block (lines `425–439`) stays unchanged immediately after.

- [ ] **Step 4: Run the figure tests — must be byte-identical green**

Run: `cargo test -p byetex-core --test captionof --test multi_label_figure 2>&1 | tail -15`
Expected: all PASS.

Run: `cargo test -p byetex-core 2>&1 | grep -E 'snapshot|FAILED|test result: FAILED' | head`
Expected: no snapshot failures. If any insta snapshot is flagged, inspect the diff — it MUST be empty (pure refactor). Do **not** `cargo insta accept`; if a snapshot changed, the refactor is wrong — fix the helper to match the original byte layout.

- [ ] **Step 5: Commit**

```bash
git add crates/byetex-core/src/emit/figures.rs
git commit -m "refactor(emit): extract emit_figure_inner (byte-identical single-figure path)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Pure column-packing helpers

Two free functions: extract a width fraction from a sub-block node, and greedily pack widths into rows to pick a column count. Pure and unit-tested in isolation.

**Files:**
- Modify: `crates/byetex-core/src/emit/figures.rs` (add two free `fn`s near the bottom, next to other `pub(in crate::emit) fn` free helpers)
- Test: `crates/byetex-core/tests/subpar_columns.rs` (create)

- [ ] **Step 1: Write the failing test**

Create `crates/byetex-core/tests/subpar_columns.rs`:

```rust
//! Column-packing heuristic for subpar.grid (Thread 5). Sub-block width
//! fractions are greedily packed into rows of cumulative width <= ~1.05; the
//! column count is the widest row's block count. No widths => single column.
use byetex_core::emit_testing::columns_for_widths;

#[test]
fn two_half_width_blocks_make_two_columns() {
    assert_eq!(columns_for_widths(&[Some(0.41), Some(0.58)]), 2);
}

#[test]
fn three_third_width_blocks_make_three_columns() {
    assert_eq!(columns_for_widths(&[Some(0.32), Some(0.32), Some(0.32)]), 3);
}

#[test]
fn third_plus_two_thirds_pack_into_two() {
    assert_eq!(columns_for_widths(&[Some(0.32), Some(0.65)]), 2);
}

#[test]
fn no_widths_is_single_column() {
    assert_eq!(columns_for_widths(&[None, None]), 1);
}

#[test]
fn overflowing_widths_wrap_to_max_row_count() {
    // 0.5 + 0.5 fills a row; the third starts a new row → max row = 2.
    assert_eq!(columns_for_widths(&[Some(0.5), Some(0.5), Some(0.5)]), 2);
}
```

To expose the pure helper to integration tests without making the whole `emit` module public, add a tiny re-export module. In `crates/byetex-core/src/lib.rs`, add near the other `pub use` lines:

```rust
/// Test-only re-exports of pure helpers (not part of the stable API).
#[doc(hidden)]
pub mod emit_testing {
    pub use crate::emit::figures::columns_for_widths;
}
```

(If `crate::emit::figures` is not already reachable, the helper is `pub(in crate::emit)`; widen `columns_for_widths` specifically to `pub` in Step 3 and reference it as `crate::emit::figures::columns_for_widths`. The module path `emit::figures` exists; `mod figures;` is declared in `emit.rs`.)

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p byetex-core --test subpar_columns 2>&1 | tail -15`
Expected: FAIL to compile — `columns_for_widths` not found.

- [ ] **Step 3: Implement the helpers**

In `figures.rs`, add near the other free helper functions at the bottom of the file:

```rust
/// Greedily pack sub-block width fractions into rows whose cumulative width is
/// <= ~1.05; return the column count = the widest row's block count. A block
/// with no width (`None`) counts as a full-width row break unless every block
/// is `None`, in which case the answer is 1 (stacked). Never returns 0.
pub fn columns_for_widths(widths: &[Option<f32>]) -> usize {
    if widths.is_empty() {
        return 1;
    }
    if widths.iter().all(|w| w.is_none()) {
        return 1;
    }
    let mut max_row = 1usize;
    let mut row_count = 0usize;
    let mut row_width = 0.0f32;
    for w in widths {
        match w {
            Some(frac) => {
                if row_count > 0 && row_width + frac > 1.05 {
                    // Start a new row.
                    row_count = 0;
                    row_width = 0.0;
                }
                row_count += 1;
                row_width += frac;
                max_row = max_row.max(row_count);
            }
            None => {
                // Unknown width → treat as its own full row.
                row_count = 0;
                row_width = 0.0;
            }
        }
    }
    max_row.max(1)
}

/// Extract a width fraction (e.g. `0.41` from `{0.41\textwidth}` /
/// `{0.5\linewidth}` / `{0.5\columnwidth}`) from a `minipage` / `subfigure` /
/// `subtable` environment node's optional size argument. Returns `None` when no
/// fraction-of-text-width argument is present.
pub fn width_fraction_of(node: tree_sitter::Node<'_>, src: &str) -> Option<f32> {
    let text = &src[node.start_byte()..node.end_byte()];
    // Find the first `{<num>\textwidth}` / `\linewidth` / `\columnwidth`.
    for unit in ["\\textwidth", "\\linewidth", "\\columnwidth"] {
        if let Some(pos) = text.find(unit) {
            // Walk back over the numeric literal preceding the unit.
            let bytes = text.as_bytes();
            let mut start = pos;
            while start > 0 {
                let c = bytes[start - 1];
                if c.is_ascii_digit() || c == b'.' {
                    start -= 1;
                } else {
                    break;
                }
            }
            if start < pos {
                if let Ok(v) = text[start..pos].parse::<f32>() {
                    return Some(v);
                }
            }
        }
    }
    None
}
```

Note the import: the file already uses `Node<'_>` from `tree_sitter` (e.g. `node: Node<'_>` parameters). Use the same `Node<'_>` type already imported rather than the fully-qualified `tree_sitter::Node<'_>` if that's the file's convention — match the existing `use tree_sitter::Node;` at the top.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p byetex-core --test subpar_columns 2>&1 | tail -15`
Expected: 5 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/byetex-core/src/emit/figures.rs crates/byetex-core/src/lib.rs crates/byetex-core/tests/subpar_columns.rs
git commit -m "feat(emit): pure column-packing helpers for subpar.grid

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Conditional `#import subpar` infrastructure

Add a `used_subpar` flag and inject the import in `finish()`. The flag will be set by Tasks 4/5; here we wire the plumbing and a `pub` test helper so the mechanism is verifiable now.

**Files:**
- Modify: `crates/byetex-core/src/emit.rs:209` (field decl), `:322` (init), `finish()` body (`~493` and `~535`)

- [ ] **Step 1: Write the failing test**

Create `crates/byetex-core/tests/subpar_import.rs`:

```rust
//! The `@preview/subpar` import is emitted exactly once, only when a
//! subpar.grid is present, and never for ordinary single-caption documents.
use byetex_core::convert;

#[test]
fn single_figure_document_has_no_subpar_import() {
    let src = "\\documentclass{article}\n\\begin{document}\n\
        \\begin{figure}\\includegraphics{x.png}\\caption{C}\\end{figure}\n\
        \\end{document}\n";
    let t = byetex_core::convert(src, &Default::default()).typst;
    assert!(
        !t.contains("@preview/subpar"),
        "single-caption doc must stay import-free; got:\n{t}"
    );
}
```

This passes today (no grid yet). Add the *real* assertion in Task 5's tests once a grid is emitted. For now this guards the negative case.

- [ ] **Step 2: Run it (sanity)**

Run: `cargo test -p byetex-core --test subpar_import 2>&1 | tail -8`
Expected: PASS (negative guard holds).

- [ ] **Step 3: Add the `used_subpar` field**

In `crates/byetex-core/src/emit.rs`, immediately after the `used_text_label_anchor: bool,` field declaration (line ~209), add:

```rust
    /// Set when a `#subpar.grid(...)` is emitted; triggers the conditional
    /// `#import "@preview/subpar:0.2.2"` at the top of the document in `finish()`.
    used_subpar: bool,
```

In the struct initializer, after `used_text_label_anchor: false,` (line ~322), add:

```rust
            used_subpar: false,
```

- [ ] **Step 4: Inject the import in `finish()`**

In the `is_document` branch of `finish()`, the preamble is built starting with `self.out.push_str(&build_neutral_preamble(...))`. Immediately **before** that push, add:

```rust
            if self.used_subpar {
                self.out.push_str("#import \"@preview/subpar:0.2.2\"\n");
            }
```

In the bare-fragment branch (where `let mut preamble = String::new();` is built), add at the very top of that block, right after `let mut preamble = String::new();`:

```rust
        if self.used_subpar {
            preamble.push_str("#import \"@preview/subpar:0.2.2\"\n");
        }
```

(The fragment branch only prepends `preamble` when it is non-empty; adding the import makes it non-empty, so a fragment that emits a grid still gets the import.)

- [ ] **Step 5: Run the full suite — nothing regresses, import still absent**

Run: `cargo test -p byetex-core --test subpar_import 2>&1 | tail -8`
Expected: PASS (still no grid emitted anywhere, so still import-free).

Run: `cargo build -p byetex-core 2>&1 | tail -3`
Expected: compiles (an `unused field used_subpar` warning is acceptable here — Task 4 uses it; if `#[warn(dead_code)]` is denied in CI it will be read in Task 4 within this same branch, so leave it).

- [ ] **Step 6: Commit**

```bash
git add crates/byetex-core/src/emit.rs crates/byetex-core/tests/subpar_import.rs
git commit -m "feat(emit): conditional @preview/subpar import infrastructure (used_subpar flag)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Pattern A — `subfigure`/`subtable` envs → `subpar.grid`

Today N subfigure panels render as a plain `grid(...)` inside one outer `#figure`, with panel labels only as hidden anchors and no sub-numbering. Upgrade: emit a `#subpar.grid(...)` with each panel as `figure(...), <panel-label>` and the float-level caption/label on the grid.

**Files:**
- Modify: `crates/byetex-core/src/emit/figures.rs` — `emit_figure` (the `panels`/`body_str` branch ~323-339 and the assembly), and `render_subfigure_panel` (~to also return the panel's label + width).
- Test: `crates/byetex-core/tests/multicaption_grid.rs` (create)

- [ ] **Step 1: Write the failing test**

Create `crates/byetex-core/tests/multicaption_grid.rs`:

```rust
//! Thread 5: a float with multiple captioned sub-blocks becomes a
//! `#subpar.grid(...)` — one inner figure per sub-block, each with its own
//! caption and label, the parent caption/label on the grid.
use byetex_core::convert;

fn typ(src: &str) -> String {
    byetex_core::convert(src, &Default::default()).typst
}

#[test]
fn subtables_with_main_caption_become_subpar_grid() {
    let t = typ(
        "\\begin{table}\n\
         \\caption{Ablations}\\label{tab:main}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{A}\\label{tab:a}\n\
         \\begin{tabular}{ll}x & y\\\\\\end{tabular}\\end{subtable}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{B}\\label{tab:b}\n\
         \\begin{tabular}{ll}p & q\\\\\\end{tabular}\\end{subtable}\n\
         \\begin{subtable}[t]{0.32\\textwidth}\\caption{C}\\label{tab:c}\n\
         \\begin{tabular}{ll}m & n\\\\\\end{tabular}\\end{subtable}\n\
         \\end{table}\n\nSee \\ref{tab:a} and \\ref{tab:main}.",
    );
    assert!(t.contains("#subpar.grid("), "expected subpar.grid; got:\n{t}");
    assert!(t.contains("columns: (1fr, 1fr, 1fr)"), "expected 3 columns; got:\n{t}");
    assert!(t.contains("caption: [Ablations]"), "parent caption on grid; got:\n{t}");
    assert!(t.contains("label: <tab:main>"), "parent label on grid; got:\n{t}");
    assert!(t.contains("<tab:a>"), "sub-label a attached; got:\n{t}");
    assert!(t.contains("caption: [A]"), "sub-caption A present; got:\n{t}");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p byetex-core --test multicaption_grid 2>&1 | tail -20`
Expected: FAIL — output contains a plain `grid(` / single `#figure`, not `#subpar.grid(`.

- [ ] **Step 3: Make `render_subfigure_panel` also yield label + width**

Change `render_subfigure_panel` to return the panel string plus its picked label and width fraction. Replace its signature and body tail:

```rust
/// One subfigure/subtable panel → (figure_string, picked_label, width_fraction).
/// The figure string has no leading `#` and no trailing `<label>`.
fn render_subfigure_panel(
    &mut self,
    node: Node<'_>,
) -> Option<(String, Option<String>, Option<f32>)> {
    let mut graphics: Option<Node<'_>> = None;
    let mut caption: Option<Node<'_>> = None;
    let mut nested_tabular: Option<Node<'_>> = None;
    let mut labels: Vec<String> = Vec::new();
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            match child.kind() {
                "graphics_include" if graphics.is_none() => graphics = Some(child),
                "caption" if caption.is_none() => caption = Some(child),
                "label_definition" => {
                    if let Some(k) = extract_label_name(child, self.src) {
                        if !labels.contains(&k) {
                            labels.push(k);
                        }
                    }
                }
                "generic_environment" => {
                    let env = environment_name(child, self.src);
                    if matches!(
                        env.as_deref(),
                        Some("tabular") | Some("tabular*") | Some("tabularx")
                            | Some("tabulary") | Some("array")
                    ) && nested_tabular.is_none()
                    {
                        nested_tabular = Some(child);
                    }
                    stack.push(child);
                }
                _ => stack.push(child),
            }
        }
    }
    // Body: image wins, else nested tabular, else nothing → drop the panel.
    let (body, is_table) = if let Some(g) = graphics {
        (self.with_sub_buffer(|e| { e.emit_graphics_include(g); }), false)
    } else if let Some(t) = nested_tabular {
        let s = self
            .with_sub_buffer(|e| { e.emit_tabular(t); })
            .trim()
            .to_string();
        (s.strip_prefix('#').map(|s| s.to_string()).unwrap_or(s), true)
    } else {
        return None;
    };
    let kind = if is_table { Some("table") } else { None };
    let caption_text = caption.and_then(|c| {
        first_curly_group(c).map(|a| self.render_curly_group_content(a))
    });
    let inner = self.emit_figure_inner(body.trim(), kind, caption_text.as_deref());
    let label = self.pick_label_to_attach(&labels);
    let width = width_fraction_of(node, self.src);
    Some((inner, label, width))
}
```

- [ ] **Step 4: Branch `emit_figure` to a subpar.grid when there are subfigure panels**

In `emit_figure`, replace the `panels` collection + the `if !panels.is_empty()` body-building branch (lines ~323-339) and route to a grid. After the discovery walk and the `includes` label harvest, insert this BEFORE the `let mut body_is_table = …; let body_str = …` chain:

```rust
        // Pattern A: explicit subfigure/subtable panels → subpar.grid.
        if subfigures.len() >= 2
            || (subfigures.len() == 1 && (caption.is_some() || captionof.is_some()))
        {
            let panels: Vec<(String, Option<String>, Option<f32>)> = subfigures
                .iter()
                .filter_map(|sf| self.render_subfigure_panel(*sf))
                .collect();
            if panels.len() >= 2 {
                // Sub-labels belong to the panels now; remove them from the
                // outer `labels` set so they are not also hidden-anchored.
                let panel_labels: std::collections::HashSet<String> = panels
                    .iter()
                    .filter_map(|(_, l, _)| l.clone())
                    .collect();
                let parent_labels: Vec<String> = labels
                    .iter()
                    .filter(|l| !panel_labels.contains(*l))
                    .cloned()
                    .collect();
                let widths: Vec<Option<f32>> = panels.iter().map(|(_, _, w)| *w).collect();
                let cols = columns_for_widths(&widths);
                let parent_kind = if environment_name(node, self.src).as_deref()
                    == Some("table")
                {
                    Some("table")
                } else {
                    None
                };
                let parent_caption = caption.or(captionof).and_then(|c| {
                    let arg = if c.kind() == "generic_command" {
                        nth_curly_group(c, 1)
                    } else {
                        first_curly_group(c)
                    };
                    arg.map(|a| self.render_curly_group_content(a))
                });
                self.emit_subpar_grid(&panels, cols, parent_kind, parent_caption.as_deref(), &parent_labels);
                return node.end_byte();
            }
        }
```

Then add the grid emitter method:

```rust
/// Emit `#subpar.grid(...)` from rendered panels `(inner_figure, label, _)`.
fn emit_subpar_grid(
    &mut self,
    panels: &[(String, Option<String>, Option<f32>)],
    cols: usize,
    parent_kind: Option<&str>,
    parent_caption: Option<&str>,
    parent_labels: &[String],
) {
    self.used_subpar = true;
    self.ensure_paragraph_break();
    self.out.push_str("#subpar.grid(\n");
    for (inner, label, _w) in panels {
        self.out.push_str("  ");
        self.out.push_str(inner);
        if let Some(l) = label {
            let _ = write!(self.out, ", <{}>", l);
        }
        self.out.push_str(",\n");
    }
    let cols_str = std::iter::repeat("1fr").take(cols).collect::<Vec<_>>().join(", ");
    let _ = write!(self.out, "  columns: ({}),\n", cols_str);
    if let Some(k) = parent_kind {
        let _ = write!(self.out, "  kind: {},\n", k);
    }
    if let Some(c) = parent_caption {
        let _ = write!(self.out, "  caption: [{}],\n", c);
    }
    let primary = self.pick_label_to_attach(parent_labels);
    if let Some(l) = &primary {
        let _ = write!(self.out, "  label: <{}>,\n", l);
    }
    self.out.push_str(")");
    // Any extra referenced parent labels get hidden anchors (existing pattern).
    for l in parent_labels {
        if Some(l) != primary.as_ref()
            && self.referenced_labels.contains(&sanitize_label_key(l))
        {
            let _ = write!(self.out, "\n#hide[#figure([]) <{}>]", l);
        }
    }
}
```

Note: `columns: (1fr)` (one element) is a parenthesised expression, not a 1-tuple, but a 1-panel grid never reaches here (guarded by `panels.len() >= 2`), so `cols >= 1` with ≥2 panels always yields a valid `(1fr, …)`. For `cols == 1` the string is `"1fr"` → `columns: (1fr)` which Typst reads as a single track — acceptable for a stacked grid.

**CRITICAL — update the leftover panels block.** Changing `render_subfigure_panel`'s return type breaks the existing call at lines `~323-339` (it built `let panels: Vec<String> = …` and used `panels.join`). That old block now only needs to handle the **single-panel / zero-panel fall-through** (the `>= 2` case is fully handled by the early-returning Pattern-A branch above). Replace the old block (`let panels: Vec<String> = subfigures … ;` through the `let body_str = if !panels.is_empty() { … }` opening arm) so the panel arm reads:

```rust
        // A lone subfigure (no main caption) collapses to just its panel as the
        // figure body; the >=2 case is handled by the Pattern-A subpar.grid
        // branch above (which returns early).
        let lone_panel: Option<String> = subfigures
            .iter()
            .filter_map(|sf| self.render_subfigure_panel(*sf))
            .map(|(inner, _label, _w)| inner)
            .next();

        let mut body_is_table = false;
        let body_str = if let Some(panel) = lone_panel {
            panel
        } else if let Some(g) = graphics {
```

i.e. delete the `panels`/`grid(columns: 2, …)` construction entirely and keep the remaining `else if let Some(g) = graphics { … }` / tabular / placeholder arms unchanged after this opening arm. Verify the resulting `body_str` chain compiles and the existing single-subfigure behavior (one panel becomes the figure body) is preserved by the snapshot check in Step 6.

- [ ] **Step 5: Run the test**

Run: `cargo test -p byetex-core --test multicaption_grid 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 6: Verify single-subfigure + no-caption cases still use the old path**

Run: `cargo test -p byetex-core 2>&1 | grep -E 'FAILED|test result: FAILED' | head`
Expected: none. Check the figure snapshots specifically did not change for single-panel figures: `cargo test -p byetex-core 2>&1 | grep -i snapshot`. If a multi-panel figure snapshot changed (it legitimately now emits subpar.grid), review the diff and `cargo insta accept` ONLY that intended change; never accept blindly.

- [ ] **Step 7: Commit**

```bash
git add crates/byetex-core/src/emit/figures.rs crates/byetex-core/tests/multicaption_grid.rs
git commit -m "feat(emit): subfigure/subtable floats render as subpar.grid (Pattern A, Thread 5)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Pattern B — top-level multi-`\captionof` → `subpar.grid`

A float with no subfigure envs but ≥2 top-level captions (each usually in a `minipage`). Group by `minipage` when present; otherwise segment the float's children linearly at each caption command.

**Files:**
- Modify: `crates/byetex-core/src/emit/figures.rs` — `emit_figure` (add a Pattern-B branch after Pattern A, before the single-figure chain)
- Test: `crates/byetex-core/tests/multicaption_grid.rs` (extend)

- [ ] **Step 1: Write the failing tests**

Append to `crates/byetex-core/tests/multicaption_grid.rs`:

```rust
#[test]
fn two_captionof_minipages_become_two_column_grid() {
    let t = typ(
        "\\begin{figure}\n\
         \\begin{minipage}{0.41\\textwidth}\\includegraphics{a.png}\n\
         \\captionof{figure}{Left}\\label{fig:a}\\end{minipage}\\hfill\n\
         \\begin{minipage}{0.58\\textwidth}\\includegraphics{b.png}\n\
         \\captionof{figure}{Right}\\label{fig:b}\\end{minipage}\n\
         \\end{figure}\n\nSee \\ref{fig:a} and \\ref{fig:b}.",
    );
    assert!(t.contains("#subpar.grid("), "expected subpar.grid; got:\n{t}");
    assert!(t.contains("columns: (1fr, 1fr)"), "expected 2 columns; got:\n{t}");
    assert!(t.contains("caption: [Left]") && t.contains("caption: [Right]"),
        "both captions present; got:\n{t}");
    assert!(t.contains("<fig:a>") && t.contains("<fig:b>"),
        "both sub-labels attached; got:\n{t}");
    assert!(t.contains("@preview/subpar"), "import emitted; got:\n{t}");
}

#[test]
fn stacked_table_then_figure_captionof_single_column() {
    let t = typ(
        "\\begin{figure}\n\
         \\begin{tabular}{ll}x & y\\\\\\end{tabular}\n\
         \\captionof{table}{Tab cap}\\label{tab:s}\n\
         \\includegraphics{z.png}\n\
         \\captionof{figure}{Fig cap}\\label{fig:s}\n\
         \\end{figure}\n\nSee \\ref{tab:s} and \\ref{fig:s}.",
    );
    assert!(t.contains("#subpar.grid("), "expected subpar.grid; got:\n{t}");
    assert!(t.contains("columns: (1fr)"), "stacked → single column; got:\n{t}");
    assert!(t.contains("caption: [Tab cap]") && t.contains("caption: [Fig cap]"),
        "both captions present; got:\n{t}");
    assert!(t.contains("kind: table"), "table sub-block keeps kind: table; got:\n{t}");
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p byetex-core --test multicaption_grid 2>&1 | tail -25`
Expected: the two new tests FAIL (single `#figure`, first caption only, no subpar.grid).

- [ ] **Step 3: Implement Pattern-B detection + segmentation**

In `emit_figure`, AFTER the Pattern-A branch and BEFORE the `let mut body_is_table` chain, add:

```rust
        // Pattern B: no subfigure envs, but >=2 top-level caption sources →
        // segment into captioned sub-blocks and emit a subpar.grid.
        {
            let blocks = self.collect_caption_blocks(node);
            if blocks.len() >= 2 {
                let widths: Vec<Option<f32>> = blocks.iter().map(|b| b.width).collect();
                let cols = columns_for_widths(&widths);
                let parent_kind = if environment_name(node, self.src).as_deref()
                    == Some("table")
                {
                    Some("table")
                } else {
                    None
                };
                let panels: Vec<(String, Option<String>, Option<f32>)> = blocks
                    .into_iter()
                    .map(|b| (b.inner, b.label, b.width))
                    .collect();
                // No parent caption/label in Pattern B (every caption belongs to
                // a sub-block); pass an empty parent label set.
                self.emit_subpar_grid(&panels, cols, parent_kind, None, &[]);
                return node.end_byte();
            }
        }
```

Add the block-collection method and a small struct. Define the struct near the top of `figures.rs` (after the `use` lines):

```rust
/// One captioned sub-block discovered in a Pattern-B float.
struct CaptionBlock {
    inner: String,         // rendered `figure(...)` (no `#`, no `<label>`)
    label: Option<String>, // picked referenced label, if any
    width: Option<f32>,    // width fraction for column packing
}
```

Add the collector method to the `impl`:

```rust
/// Collect captioned sub-blocks of a Pattern-B float. Prefers `minipage`
/// grouping (each minipage that contains a caption is one block); falls back
/// to linear segmentation where each `\caption`/`\captionof` closes the run of
/// preceding sibling content. Returns empty / single when the float is not a
/// multi-caption float (caller then uses the single-figure path).
fn collect_caption_blocks(&mut self, node: Node<'_>) -> Vec<CaptionBlock> {
    // Gather the float's direct children in source order.
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node.children(&mut cursor).collect();

    // Path 1: minipage grouping.
    let minipages: Vec<Node<'_>> = children
        .iter()
        .copied()
        .filter(|c| {
            c.kind() == "generic_environment"
                && environment_name(*c, self.src).as_deref() == Some("minipage")
        })
        .collect();
    let captioned_minipages: Vec<Node<'_>> = minipages
        .iter()
        .copied()
        .filter(|mp| self.subtree_has_caption(*mp))
        .collect();
    if captioned_minipages.len() >= 2 {
        return captioned_minipages
            .iter()
            .filter_map(|mp| self.render_caption_block(*mp))
            .collect();
    }

    // Path 2: linear segmentation by caption command.
    // A caption is a `caption` node or a `\captionof` generic_command.
    let is_caption = |c: &Node<'_>| -> bool {
        c.kind() == "caption"
            || (c.kind() == "generic_command"
                && command_name_text(*c, self.src).as_deref() == Some("\\captionof"))
    };
    let caption_count = children.iter().filter(|c| is_caption(c)).count();
    if caption_count < 2 {
        return Vec::new();
    }
    let mut blocks: Vec<CaptionBlock> = Vec::new();
    let mut run: Vec<Node<'_>> = Vec::new();
    let mut run_labels: Vec<String> = Vec::new();
    for c in &children {
        if c.kind() == "label_definition" {
            if let Some(k) = extract_label_name(*c, self.src) {
                run_labels.push(k);
            }
            continue;
        }
        if is_caption(c) {
            // Close the current run as a block captioned by `c`.
            if let Some(b) = self.render_linear_block(&run, *c, &run_labels) {
                blocks.push(b);
            }
            run.clear();
            run_labels.clear();
        } else {
            run.push(*c);
        }
    }
    blocks
}

/// True if `node`'s subtree contains a `caption` node or a `\captionof`.
fn subtree_has_caption(&self, node: Node<'_>) -> bool {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            if child.kind() == "caption"
                || (child.kind() == "generic_command"
                    && command_name_text(child, self.src).as_deref()
                        == Some("\\captionof"))
            {
                return true;
            }
            stack.push(child);
        }
    }
    false
}

/// Render a minipage (or any captioned container) as one CaptionBlock: its body
/// (image or tabular), its own caption + label, and its width fraction.
fn render_caption_block(&mut self, node: Node<'_>) -> Option<CaptionBlock> {
    let mut graphics: Option<Node<'_>> = None;
    let mut caption: Option<Node<'_>> = None;
    let mut captionof: Option<Node<'_>> = None;
    let mut nested_tabular: Option<Node<'_>> = None;
    let mut labels: Vec<String> = Vec::new();
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            match child.kind() {
                "graphics_include" if graphics.is_none() => graphics = Some(child),
                "caption" if caption.is_none() => caption = Some(child),
                "generic_command"
                    if captionof.is_none()
                        && command_name_text(child, self.src).as_deref()
                            == Some("\\captionof") =>
                {
                    captionof = Some(child);
                }
                "label_definition" => {
                    if let Some(k) = extract_label_name(child, self.src) {
                        if !labels.contains(&k) {
                            labels.push(k);
                        }
                    }
                }
                "generic_environment" => {
                    let env = environment_name(child, self.src);
                    if matches!(
                        env.as_deref(),
                        Some("tabular") | Some("tabular*") | Some("tabularx")
                            | Some("tabulary") | Some("array")
                    ) && nested_tabular.is_none()
                    {
                        nested_tabular = Some(child);
                    }
                    stack.push(child);
                }
                _ => stack.push(child),
            }
        }
    }
    let (body, is_table) = if let Some(g) = graphics {
        (self.with_sub_buffer(|e| { e.emit_graphics_include(g); }), false)
    } else if let Some(t) = nested_tabular {
        let s = self.with_sub_buffer(|e| { e.emit_tabular(t); }).trim().to_string();
        (s.strip_prefix('#').map(|s| s.to_string()).unwrap_or(s), true)
    } else {
        return None;
    };
    let cap_node = caption.or(captionof);
    let kind = self.captionof_kind(captionof).or(if is_table { Some("table") } else { None });
    let caption_text = cap_node.and_then(|c| {
        let arg = if c.kind() == "generic_command" {
            nth_curly_group(c, 1)
        } else {
            first_curly_group(c)
        };
        arg.map(|a| self.render_curly_group_content(a))
    });
    let inner = self.emit_figure_inner(body.trim(), kind, caption_text.as_deref());
    Some(CaptionBlock {
        inner,
        label: self.pick_label_to_attach(&labels),
        width: width_fraction_of(node, self.src),
    })
}

/// Render a linear run of content nodes + a closing caption node as a block.
fn render_linear_block(
    &mut self,
    run: &[Node<'_>],
    caption: Node<'_>,
    labels: &[String],
) -> Option<CaptionBlock> {
    // Render the run's content into a sub-buffer (images, tabulars, text).
    let body = self.with_sub_buffer(|e| {
        for n in run {
            e.emit_node_public(*n);
        }
    });
    let body = body.trim().to_string();
    let body = body.strip_prefix('#').map(|s| s.to_string()).unwrap_or(body);
    if body.is_empty() {
        return None;
    }
    let is_table = body.starts_with("table(");
    let kind = self
        .captionof_kind(if caption.kind() == "generic_command" { Some(caption) } else { None })
        .or(if is_table { Some("table") } else { None });
    let arg = if caption.kind() == "generic_command" {
        nth_curly_group(caption, 1)
    } else {
        first_curly_group(caption)
    };
    let caption_text = arg.map(|a| self.render_curly_group_content(a));
    let inner = self.emit_figure_inner(&body, kind, caption_text.as_deref());
    Some(CaptionBlock {
        inner,
        label: self.pick_label_to_attach(labels),
        width: None, // linear/stacked → no width → single column
    })
}

/// `\captionof{type}{...}` → `Some("table")` / `Some("image")` / `None`.
fn captionof_kind(&mut self, captionof: Option<Node<'_>>) -> Option<&'static str> {
    let c = captionof?;
    let type_arg = nth_curly_group(c, 0)?;
    let ty = self.render_curly_group_content(type_arg);
    match ty.trim() {
        "table" => Some("table"),
        "figure" => Some("image"),
        _ => None,
    }
}
```

This references two things that must exist:
1. `command_name_text(node, src)` — already used in `emit_figure` (`command_name_text(child, self.src)`); it is imported in `figures.rs`. Reuse it.
2. `emit_node_public` — a thin wrapper to call the private `emit_node` from `figures.rs` on arbitrary nodes inside `with_sub_buffer`. Check whether an existing public-ish dispatch exists. If `emit_node` is callable from `figures.rs` (same crate, `impl Emitter`), call `e.emit_node(*n)` directly instead of `emit_node_public`. Inspect `emit.rs` for the `emit_node` visibility; it is a method on `Emitter` (private `fn emit_node`). Since `figures.rs` is `mod figures` inside `emit`, `self.emit_node(...)` is reachable. Use `e.emit_node(*n)` and delete the `emit_node_public` reference.

- [ ] **Step 4: Run the Pattern-B tests**

Run: `cargo test -p byetex-core --test multicaption_grid 2>&1 | tail -25`
Expected: all PASS (Pattern A from Task 4 + the two new Pattern B tests).

- [ ] **Step 5: Strengthen the import test**

Append to `crates/byetex-core/tests/subpar_import.rs`:

```rust
#[test]
fn grid_document_imports_subpar_exactly_once() {
    let src = "\\documentclass{article}\n\\begin{document}\n\
        \\begin{figure}\n\
        \\begin{minipage}{0.5\\textwidth}\\includegraphics{a.png}\\captionof{figure}{L}\\label{f:a}\\end{minipage}\n\
        \\begin{minipage}{0.5\\textwidth}\\includegraphics{b.png}\\captionof{figure}{R}\\label{f:b}\\end{minipage}\n\
        \\end{figure}\n\\end{document}\n";
    let t = byetex_core::convert(src, &Default::default()).typst;
    assert_eq!(t.matches("@preview/subpar").count(), 1, "import exactly once; got:\n{t}");
    assert!(t.trim_start().starts_with("#import \"@preview/subpar"),
        "import must be at the very top; got:\n{}", &t[..t.len().min(200)]);
}
```

Run: `cargo test -p byetex-core --test subpar_import 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 6: Full suite + clippy + snapshots**

Run: `cargo test --workspace 2>&1 | grep -E 'FAILED|test result: FAILED' | head`
Expected: none. Review any flagged figure snapshot diffs; accept only intended subpar.grid changes (`cargo insta accept` on the specific reviewed snapshot, never blind).

Run: `cargo clippy -p byetex-core --lib 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/byetex-core/src/emit/figures.rs crates/byetex-core/tests/multicaption_grid.rs crates/byetex-core/tests/subpar_import.rs
git commit -m "feat(emit): top-level multi-captionof floats render as subpar.grid (Pattern B, Thread 5)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Corpus verification + acceptance gate

Confirm the corpus drivers still compile (now via subpar) and that captions are no longer dropped. No new code — this is the verification gate.

**Files:** none (verification only). If a regression appears, fix it in `figures.rs` and re-run.

- [ ] **Step 1: Build the worktree binary**

Run: `cargo build --release -p byetex-cli 2>&1 | tail -2`
Expected: Finished.

- [ ] **Step 2: Diagnose the three drivers — they must compile (0 typst errors)**

```bash
BIN=target/release/byetex
for p in 2605.22507 2605.31063 2605.31604; do
  $BIN diagnose --project "corpus/$p/source/"*.tex 2>&1 | tail -1
done
```
Expected: each prints `0 typst error(s)` (or the existing per-paper count if a driver had unrelated pre-existing errors — compare against `main`; the NUMBER must not increase). If a paper newly fails, read its `*.diagnostics.json`, fix `figures.rs`, rebuild, repeat.

Note: `2605.31063` and `2605.31604` are `INPUT_BROKEN` in the current baseline (the LaTeX itself doesn't compile under tectonic) — for those, the gate only requires that **byetex still produces output without a NEW byetex-attributable error**. `2605.22507` is `known_pass` and must stay compiling.

- [ ] **Step 3: Run the acceptance gate — no compile regression**

```bash
export BYETEX_BIN="$(pwd)/target/release/byetex"
bash scripts/acceptance.sh 2>&1 | tail -6
```
Expected: `acceptance: PASS=45 BYETEX_FAIL=0` with `OK: no compile regression in known_pass set.` If any `known_pass` paper regresses, fix before proceeding.

- [ ] **Step 4: Visual spot-check (captions now visible)**

```bash
uv run --with requests --with Pillow python scripts/visual_test.py 2605.22507 2>&1 | tail -20
```
Expected: the composite PNG for 2605.22507 shows BOTH the table caption and the figure caption (previously only one). Eyeball the generated `.typ` to confirm a `#subpar.grid(` is present for the multi-caption float.

- [ ] **Step 5: Commit any fixes (if Steps 2-4 required code changes)**

```bash
git add crates/byetex-core/src/emit/figures.rs
git commit -m "fix(emit): corpus-driven subpar.grid fixes (Thread 5 verification)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```
(Skip if no changes were needed.)

---

## Final verification (after all tasks)

- `cargo test --workspace` fully green; only intended figure-snapshot changes accepted.
- `cargo clippy -p byetex-core --lib` clean.
- `BYETEX_BIN="$(pwd)/target/release/byetex" bash scripts/acceptance.sh` → PASS 45 / BYETEX_FAIL 0.
- Single-caption floats unchanged (captionof.rs, multi_label_figure.rs green; figure snapshots byte-identical except intended multi-panel ones).
- Open the PR: title `feat(emit): split multi-caption floats into subpar.grid (Thread 5)`, body summarising the two patterns, the conditional import tradeoff, and the corpus drivers. Run the acceptance gate before merging.
