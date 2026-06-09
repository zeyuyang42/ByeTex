# AI fallback / scoped fragment repair — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give an external AI agent a CLI repair loop — `byetex diagnose` compiles the
generated Typst, maps each typst error back to the originating LaTeX fragment, and names the
skill that explains the fix.

**Architecture:** Tools-only, CLI-primary (the agent has a shell; the MCP is not touched).
One new core capability — a **content-anchored** source map (each emitted node records the
`.typ` text it produced + its LaTeX source span) — plus a `typst` stderr parser, a
category→skill table, a `byetex diagnose` subcommand, and a repair-loop skill/doc.

**Tech Stack:** Rust (workspace: `byetex-core`, `byetex-cli`); `typst` CLI (shelled out, as
`byetex doctor`/`agent-brief` already do); the build-time skill catalogue under `skills/`.

**Spec:** `docs/superpowers/specs/2026-06-09-ai-fallback-repair-design.md`.

**Workflow:** This branch is `design/ai-fallback-repair` (already created). One commit per
task. After each task: `cargo test --workspace` green and `cargo clippy -p byetex-core -p byetex-cli`
clean before moving on. The source-map capture is **gated off by default**, so existing
goldens/snapshots must stay byte-identical (Task 4 verifies this explicitly).

---

## File structure

| File | New/Mod | Responsibility |
|------|---------|----------------|
| `crates/byetex-core/src/typst_diag.rs` | new | Parse `typst compile` stderr → `Vec<TypstError>`. Pure. |
| `crates/byetex-core/src/source_map.rs` | new | `NodeOutput` type + `resolve_error_line`. Pure. |
| `crates/byetex-core/src/skill_map.rs` | new | `default_skill_for(&Category) -> Option<&'static str>`. Pure. |
| `crates/byetex-core/src/warnings.rs` | mod | (nothing structural — `Category` already exists; referenced by `skill_map`). |
| `crates/byetex-core/src/emit.rs` | mod | `Emitter.record_source_map`/`.source_map` fields; `emit_node` wrapper; `finish()` fills `suggested_skill` via `default_skill_for` and returns the map. |
| `crates/byetex-core/src/lib.rs` | mod | `mod` declarations + re-exports; `ConvertOutput.source_map`; `convert_capturing_source_map`. |
| `crates/byetex-cli/src/main.rs` | mod | `Command::Diagnose` + `run_diagnose`. |
| `skills/byetex-repair-loop.md` | new | The init doc: the exact repair loop for an agent. |
| `docs/for-agents.md` | mod | Add the loop diagram. |
| `crates/byetex-core/tests/{typst_diag,source_map,default_skill}.rs` | new | Unit tests. |
| `crates/byetex-cli/tests/diagnose.rs` | new | CLI integration test. |

---

## Task 1: `default_skill_for` — category → skill table, wired into warnings

**Files:**
- Create: `crates/byetex-core/src/skill_map.rs`
- Modify: `crates/byetex-core/src/lib.rs` (add `mod skill_map; pub use skill_map::default_skill_for;`)
- Modify: `crates/byetex-core/src/emit.rs` (`finish()` — fill `suggested_skill` when `None`)
- Test: `crates/byetex-core/tests/default_skill.rs`

- [ ] **Step 1: Write the failing unit test for the mapping**

Create `crates/byetex-core/tests/default_skill.rs`:

```rust
use byetex_core::{default_skill_for, skills, Category};

#[test]
fn each_category_maps_to_an_existing_skill() {
    let cases = [
        Category::UnsupportedEnvironment { name: "x".into() },
        Category::Tikz,
        Category::CustomMacro { name: "x".into() },
        Category::ParseError { tree_sitter_node: "x".into() },
        Category::AmbiguousMath { reason: "x".into() },
        Category::UnsupportedCommand { name: "x".into() },
        Category::NeedsManualReview { reason: "x".into() },
    ];
    for cat in cases {
        let name = default_skill_for(&cat).expect("every category should map to a skill");
        assert!(
            skills::read_skill(name).is_some(),
            "skill `{name}` for {cat:?} must exist in the catalogue"
        );
    }
}

#[test]
fn tikz_maps_to_tikz_skill() {
    assert_eq!(default_skill_for(&Category::Tikz), Some("byetex-tikz-to-typst"));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p byetex-core --test default_skill`
Expected: FAIL — `default_skill_for` is not defined / not exported.

- [ ] **Step 3: Implement `skill_map.rs`**

Create `crates/byetex-core/src/skill_map.rs`:

```rust
//! Default warning-category → repair-skill mapping. Used to fill a warning's
//! `suggested_skill` when an emit site didn't set one explicitly, so every
//! warning points an agent at the guide that explains the fix.

use crate::warnings::Category;

/// The skill name that best explains how to act on a warning of this category.
/// Returns `None` only for categories with no actionable guide.
pub fn default_skill_for(cat: &Category) -> Option<&'static str> {
    match cat {
        Category::UnsupportedEnvironment { .. } => Some("byetex-unsupported-environment"),
        Category::Tikz => Some("byetex-tikz-to-typst"),
        Category::CustomMacro { .. } => Some("byetex-custom-macros"),
        Category::ParseError { .. } => Some("byetex-parse-error"),
        Category::AmbiguousMath { .. } => Some("byetex-using-warnings-json"),
        Category::UnsupportedCommand { .. } => Some("byetex-using-warnings-json"),
        Category::NeedsManualReview { .. } => Some("byetex-using-warnings-json"),
        Category::UnknownPackage { .. } => Some("byetex-using-warnings-json"),
        Category::DropOnly { .. } => None,
    }
}
```

Add to `crates/byetex-core/src/lib.rs` (near the other `mod`/`pub use` lines, e.g. by
`pub use warnings::{...}` at line ~41):

```rust
mod skill_map;
pub use skill_map::default_skill_for;
```

(`skills` is already public — `pub mod skills;` / `pub use`. Confirm `Category` is re-exported;
it is, via `pub use warnings::{Category, Range, Severity, Warning};`.)

- [ ] **Step 4: Run the unit test to verify it passes**

Run: `cargo test -p byetex-core --test default_skill`
Expected: PASS (both tests).

- [ ] **Step 5: Write the failing wiring test**

Append to `crates/byetex-core/tests/default_skill.rs`:

```rust
#[test]
fn unsupported_env_warning_gets_suggested_skill_filled() {
    let out = byetex_core::convert(
        r"\begin{flushleft}hi\end{flushleft}",
        &Default::default(),
    );
    // `flushleft` is unsupported → an unsupported_environment warning whose
    // suggested_skill must be auto-filled from the category.
    let w = out
        .warnings
        .iter()
        .find(|w| matches!(&w.category, Category::UnsupportedEnvironment { .. }))
        .expect("expected an unsupported_environment warning");
    assert_eq!(w.suggested_skill.as_deref(), Some("byetex-unsupported-environment"));
}
```

Run: `cargo test -p byetex-core --test default_skill unsupported_env_warning_gets_suggested_skill_filled`
Expected: FAIL — `suggested_skill` is `None` (warn sites don't set it; nothing fills it yet).
(If `flushleft` happens to be supported, pick another unsupported env by checking
`crates/byetex-core/src/emit/environments.rs::warn_unsupported_environment`.)

- [ ] **Step 6: Fill `suggested_skill` in `finish()`**

In `crates/byetex-core/src/emit.rs`, inside `finish()` (starts line ~460), just before the
`warnings` are returned (after all emission/backstops, near the final `return`/tuple at the
end of the function), add:

```rust
// Fill each warning's suggested_skill from its category when an emit site
// didn't set one explicitly, so every warning points at a repair guide.
for w in &mut self.warnings {
    if w.suggested_skill.is_none() {
        w.suggested_skill = crate::skill_map::default_skill_for(&w.category).map(str::to_string);
    }
}
```

(`self.warnings` is the `Vec<Warning>` returned by `finish()`. Locate the line that moves
warnings into the return value — e.g. `let warnings = std::mem::take(&mut self.warnings);` —
and put this loop immediately before it, operating on `self.warnings`.)

- [ ] **Step 7: Run the wiring test + full suite**

Run: `cargo test -p byetex-core --test default_skill`
Expected: PASS (all three tests).
Run: `cargo test --workspace`
Expected: PASS. Note: filling previously-`None` `suggested_skill` values changes
`warnings.json` content; if any golden/snapshot embeds a warning's `suggested_skill`, update
that golden intentionally (review the diff — it should only flip `null` → a skill name).

- [ ] **Step 8: Wire the bibliography-specific skill (spec item)**

Bibliography warnings are raised as `Category::NeedsManualReview` (so `default_skill_for`
gives them the generic `byetex-using-warnings-json`). Point them at `byetex-bibliography`
instead. In `crates/byetex-core/src/emit/bibliography.rs`, find each `self.warnings.push(Warning { ... })`
whose `category` is `NeedsManualReview` about a `.bib`/`.bbl`/`\bibliography` (e.g. the
"references missing file" and ".bbl inlined as fallback" warnings) and set
`suggested_skill: Some("byetex-bibliography".to_string())` on them (replacing the `None`).
Add a test in `crates/byetex-core/tests/default_skill.rs`:

```rust
#[test]
fn bibliography_warning_suggests_bibliography_skill() {
    use std::fs;
    let dir = std::env::temp_dir().join(format!("byetex-bibskill-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("p.tex"),
        "\\documentclass{article}\\begin{document}x\\bibliography{NoSuchFile}\\end{document}").unwrap();
    let out = byetex_core::convert(
        &fs::read_to_string(dir.join("p.tex")).unwrap(),
        &byetex_core::ConvertOptions { source_name: Some("p.tex".into()), base_dir: Some(dir.clone()) },
    );
    assert!(
        out.warnings.iter().any(|w| w.suggested_skill.as_deref() == Some("byetex-bibliography")),
        "a missing-.bib warning should suggest the bibliography skill; got {:?}",
        out.warnings
    );
    let _ = fs::remove_dir_all(&dir);
}
```

Run: `cargo test -p byetex-core --test default_skill bibliography_warning_suggests_bibliography_skill`
Expected: PASS after setting the skill at the bib emit sites. (`default_skill_for`'s loop in
Step 6 only fills `None`s, so an explicit `Some("byetex-bibliography")` at the site is
preserved.)

- [ ] **Step 9: Commit**

```bash
git add crates/byetex-core/src/skill_map.rs crates/byetex-core/src/lib.rs \
        crates/byetex-core/src/emit.rs crates/byetex-core/src/emit/bibliography.rs \
        crates/byetex-core/tests/default_skill.rs
git commit -m "feat(core): default_skill_for + fill warning.suggested_skill from category"
```

---

## Task 2: `typst_diag` — parse `typst compile` stderr into structured errors

**Files:**
- Create: `crates/byetex-core/src/typst_diag.rs`
- Modify: `crates/byetex-core/src/lib.rs` (`mod typst_diag; pub use typst_diag::{parse_typst_errors, TypstError};`)
- Test: `crates/byetex-core/tests/typst_diag.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/byetex-core/tests/typst_diag.rs`:

```rust
use byetex_core::parse_typst_errors;

const STDERR: &str = "\
error: unknown variable: arrival
  ┌─ main.typ:134:0
  │
134 │ P(B_(tau_i)|arrival)
  │
error: unexpected argument
  ┌─ main.typ:200:12
";

#[test]
fn parses_message_line_col_for_each_error() {
    let errs = parse_typst_errors(STDERR);
    assert_eq!(errs.len(), 2);
    assert_eq!(errs[0].message, "unknown variable: arrival");
    assert_eq!(errs[0].line, 134);
    assert_eq!(errs[0].col, 0);
    assert_eq!(errs[1].message, "unexpected argument");
    assert_eq!(errs[1].line, 200);
    assert_eq!(errs[1].col, 12);
}

#[test]
fn empty_stderr_yields_no_errors() {
    assert!(parse_typst_errors("").is_empty());
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p byetex-core --test typst_diag`
Expected: FAIL — `parse_typst_errors` not defined.

- [ ] **Step 3: Implement `typst_diag.rs`**

Create `crates/byetex-core/src/typst_diag.rs`:

```rust
//! Parse `typst compile` stderr into structured errors. Pure (no process
//! spawning) so it is unit-testable without the typst binary. Typst's
//! diagnostic format is:
//!     error: <message>
//!       ┌─ <file>:<line>:<col>
//! (optionally followed by source-snippet lines we ignore).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypstError {
    pub message: String,
    /// 1-based line in the `.typ`, as typst reports.
    pub line: usize,
    /// 0-based column, as typst reports.
    pub col: usize,
}

/// Extract every `error:` diagnostic with a location line. Diagnostics without
/// a `┌─ file:line:col` location line are skipped (they can't be mapped).
pub fn parse_typst_errors(stderr: &str) -> Vec<TypstError> {
    let lines: Vec<&str> = stderr.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(msg) = lines[i].trim_start().strip_prefix("error: ") {
            // Look at the next line for the `┌─ file:line:col` location.
            if let Some(loc) = lines.get(i + 1).and_then(|l| parse_location(l)) {
                out.push(TypstError {
                    message: msg.trim().to_string(),
                    line: loc.0,
                    col: loc.1,
                });
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Parse a `  ┌─ main.typ:134:0` line into `(line, col)`. The box-drawing
/// prefix varies; key on the trailing `:<line>:<col>`.
fn parse_location(line: &str) -> Option<(usize, usize)> {
    let after = line.rsplit("─ ").next()?; // text after the box-drawing rule
    // after looks like `main.typ:134:0`
    let mut parts = after.rsplitn(3, ':');
    let col: usize = parts.next()?.trim().parse().ok()?;
    let ln: usize = parts.next()?.trim().parse().ok()?;
    parts.next()?; // the path (ignored)
    Some((ln, col))
}
```

Add to `crates/byetex-core/src/lib.rs`:

```rust
mod typst_diag;
pub use typst_diag::{parse_typst_errors, TypstError};
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test -p byetex-core --test typst_diag`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add crates/byetex-core/src/typst_diag.rs crates/byetex-core/src/lib.rs \
        crates/byetex-core/tests/typst_diag.rs
git commit -m "feat(core): typst_diag — parse typst compile stderr into structured errors"
```

---

## Task 3: source-map type + `resolve_error_line` (pure)

**Files:**
- Create: `crates/byetex-core/src/source_map.rs`
- Modify: `crates/byetex-core/src/lib.rs` (`mod source_map; pub use source_map::{NodeOutput, resolve_error_line};`)
- Test: `crates/byetex-core/tests/source_map.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/byetex-core/tests/source_map.rs`:

```rust
use byetex_core::{resolve_error_line, NodeOutput};

fn n(src: (usize, usize), out: &str) -> NodeOutput {
    NodeOutput { src, output: out.to_string() }
}

#[test]
fn shortest_containing_output_wins() {
    // A parent node (whole doc) and a child node both contain the line; the
    // child (shorter output) is the more specific match.
    let map = vec![
        n((0, 100), "= Heading\n\nP(B_(tau_i)|arrival)\n"), // parent
        n((40, 60), "P(B_(tau_i)|arrival)"),                 // the math node
    ];
    let span = resolve_error_line(&map, "P(B_(tau_i)|arrival)");
    assert_eq!(span, Some((40, 60)));
}

#[test]
fn whitespace_is_normalized() {
    let map = vec![n((5, 9), "a + b")];
    assert_eq!(resolve_error_line(&map, "   a + b   "), Some((5, 9)));
}

#[test]
fn token_fallback_when_no_full_line_match() {
    // post-processing changed the line slightly; fall back to the longest token.
    let map = vec![n((3, 8), "#hide[$arrival$]")];
    assert_eq!(resolve_error_line(&map, "(#hide[$arrival$])"), Some((3, 8)));
}

#[test]
fn no_match_returns_none() {
    let map = vec![n((0, 4), "abcd")];
    assert_eq!(resolve_error_line(&map, "totally unrelated zzz"), None);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p byetex-core --test source_map`
Expected: FAIL — `NodeOutput` / `resolve_error_line` not defined.

- [ ] **Step 3: Implement `source_map.rs`**

Create `crates/byetex-core/src/source_map.rs`:

```rust
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

/// Resolve a `.typ` error line to its originating LaTeX source span. Among the
/// nodes whose (normalized) output CONTAINS the (normalized) line, returns the
/// `src` of the one with the SHORTEST output (most specific). Falls back to the
/// node whose output contains the line's longest non-whitespace token. `None`
/// if nothing matches.
pub fn resolve_error_line(map: &[NodeOutput], typ_line: &str) -> Option<(usize, usize)> {
    let needle = normalize(typ_line);
    if needle.is_empty() {
        return None;
    }
    // Pass 1: nodes whose output contains the whole line. Shortest output wins.
    let full = map
        .iter()
        .filter(|n| normalize(&n.output).contains(&needle))
        .min_by_key(|n| n.output.len());
    if let Some(n) = full {
        return Some(n.src);
    }
    // Pass 2: token fallback — longest non-whitespace token of the line.
    let token = needle.split_whitespace().max_by_key(|t| t.len())?;
    if token.len() < 3 {
        return None; // too short to anchor on reliably
    }
    map.iter()
        .filter(|n| n.output.contains(token))
        .min_by_key(|n| n.output.len())
        .map(|n| n.src)
}
```

Add to `crates/byetex-core/src/lib.rs`:

```rust
mod source_map;
pub use source_map::{resolve_error_line, NodeOutput};
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test -p byetex-core --test source_map`
Expected: PASS (all four tests).

- [ ] **Step 5: Commit**

```bash
git add crates/byetex-core/src/source_map.rs crates/byetex-core/src/lib.rs \
        crates/byetex-core/tests/source_map.rs
git commit -m "feat(core): content-anchored source-map type + resolve_error_line"
```

---

## Task 4: capture the source map during conversion (gated)

**Files:**
- Modify: `crates/byetex-core/src/emit.rs` (`Emitter` struct ~line 75; constructors ~238/253; `emit_node` ~665; `finish()` ~460)
- Modify: `crates/byetex-core/src/lib.rs` (`ConvertOutput.source_map`; `convert_capturing_source_map`; `convert_with_macros` threads the flag)
- Test: `crates/byetex-core/tests/source_map.rs` (append integration cases)

- [ ] **Step 1: Write the failing integration test**

Append to `crates/byetex-core/tests/source_map.rs`:

```rust
use byetex_core::{convert, convert_capturing_source_map, resolve_error_line as resolve, ConvertOptions};

#[test]
fn default_convert_has_empty_source_map_and_unchanged_output() {
    let src = r"\section{Intro}\nHello world.";
    let plain = convert(src, &ConvertOptions::default());
    let mapped = convert_capturing_source_map(src, &ConvertOptions::default());
    assert!(plain.source_map.is_empty(), "default convert must not capture a map");
    assert_eq!(plain.typst, mapped.typst, "capture must not change the output");
}

#[test]
fn captured_map_resolves_a_body_line_to_its_source() {
    let src = "\\section{Intro}\n\nThe quick brown fox.\n";
    let out = convert_capturing_source_map(src, &ConvertOptions::default());
    assert!(!out.source_map.is_empty());
    // A line of the emitted body resolves back into the LaTeX source range that
    // produced it (the paragraph text span).
    let span = resolve(&out.source_map, "The quick brown fox.").expect("should resolve");
    let frag = &src[span.0..span.1];
    assert!(frag.contains("quick brown fox"), "resolved fragment was: {frag:?}");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p byetex-core --test source_map`
Expected: FAIL — `convert_capturing_source_map` and `ConvertOutput.source_map` don't exist.

- [ ] **Step 3: Add the `Emitter` fields**

In `crates/byetex-core/src/emit.rs`, in `struct Emitter<'a>` (line ~75), add two fields:

```rust
    /// When true, `emit_node` records each node's output text + source span
    /// into `source_map`. Off by default (zero-overhead normal conversion).
    pub(crate) record_source_map: bool,
    /// Content-anchored provenance entries (see source_map.rs). Empty unless
    /// `record_source_map` is set.
    pub(crate) source_map: Vec<crate::source_map::NodeOutput>,
```

Initialize them in BOTH constructors `with_includes` (~238) and
`with_includes_and_macros` (~253) — wherever the `Emitter { ... }` literal is built, add:

```rust
            record_source_map: false,
            source_map: Vec::new(),
```

(If `with_includes` delegates to `with_includes_and_macros`, only the latter's literal needs
the fields. Find every `Emitter {` literal and add them.)

- [ ] **Step 4: Wrap `emit_node`**

In `crates/byetex-core/src/emit.rs`, rename the existing `fn emit_node` (line ~665) to
`fn emit_node_inner`, and add a thin wrapper directly above it:

```rust
    fn emit_node(&mut self, node: Node<'_>) -> usize {
        if !self.record_source_map {
            return self.emit_node_inner(node);
        }
        let out_start = self.out.len();
        let src = (node.start_byte(), node.end_byte());
        let r = self.emit_node_inner(node);
        if self.out.len() > out_start {
            self.source_map.push(crate::source_map::NodeOutput {
                src,
                output: self.out[out_start..].to_string(),
            });
        }
        r
    }

    fn emit_node_inner(&mut self, node: Node<'_>) -> usize {
        // ... existing body unchanged ...
```

(Only the signature line is renamed; the body is untouched. All existing callers keep
calling `emit_node`.)

- [ ] **Step 5: Return the map from `finish()` and thread the flag**

In `finish()` (line ~460), the function currently returns a 4-tuple
`(typst, warnings, asset_refs, class_metadata)` (see the destructure at `lib.rs:120`). Take
the captured map and append it as a 5th element, leaving the first four expressions exactly
as they are:

```rust
        // ... existing code that produces the four return values ...
        let source_map = std::mem::take(&mut self.source_map);
        (typst, warnings, asset_refs, class_metadata, source_map)   // <typst..class_metadata> unchanged
```

Update the `fn finish(...)` return-type signature to add the 5th element:
`, Vec<crate::source_map::NodeOutput>`.

In `crates/byetex-core/src/lib.rs`:

(a) Add the field to `ConvertOutput` (after `class_metadata`, line ~66):

```rust
    /// Content-anchored provenance map (`.typ` text → LaTeX source span) per
    /// emitted node. Empty unless produced via `convert_capturing_source_map`.
    pub source_map: Vec<source_map::NodeOutput>,
```

(b) Change `convert_with_macros` (line ~98) to take a `record_source_map: bool` param,
set it on the emitter before `prepass_collect`, capture the 5th tuple element, and put it on
`ConvertOutput`:

```rust
pub(crate) fn convert_with_macros(
    source: &str,
    opts: &ConvertOptions,
    preseeded_macros: HashMap<String, emit::MacroDef>,
    preseeded_refs: HashSet<String>,
    record_source_map: bool,
) -> ConvertOutput {
    // ... unchanged setup ...
    emitter.record_source_map = record_source_map;
    emitter.seed_referenced_labels(preseeded_refs);
    let root = tree.root_node();
    emitter.prepass_collect(root);
    emitter.emit_root(root);
    let (typst, warnings, asset_refs, class_metadata, source_map) = emitter.finish();
    ConvertOutput { typst, warnings, asset_refs, class_metadata, source_map }
}
```

(c) Update the existing `convert` to pass `false`, and add the capturing entry point:

```rust
pub fn convert(source: &str, opts: &ConvertOptions) -> ConvertOutput {
    convert_with_macros(source, opts, HashMap::new(), HashSet::new(), false)
}

/// Like [`convert`], but also records the content-anchored source map on the
/// returned `ConvertOutput.source_map`. Used by `byetex diagnose`.
pub fn convert_capturing_source_map(source: &str, opts: &ConvertOptions) -> ConvertOutput {
    convert_with_macros(source, opts, HashMap::new(), HashSet::new(), true)
}
```

(d) Update any OTHER caller of `convert_with_macros` (e.g. in `project.rs`) to pass `false`
as the new 5th argument. Run `grep -rn 'convert_with_macros(' crates/byetex-core/src` and fix
each call site.

- [ ] **Step 6: Run the integration tests + full suite**

Run: `cargo test -p byetex-core --test source_map`
Expected: PASS (all six tests now).
Run: `cargo test --workspace`
Expected: PASS. The default `convert` path is unchanged (flag off) → all goldens/snapshots
byte-identical. If anything fails, the wrapper or the flag threading is wrong — fix before
committing (do NOT `cargo insta accept`).

- [ ] **Step 7: Commit**

```bash
git add crates/byetex-core/src/emit.rs crates/byetex-core/src/lib.rs \
        crates/byetex-core/tests/source_map.rs
git commit -m "feat(core): capture content-anchored source map during conversion (gated)"
```

---

## Task 5: `byetex diagnose` CLI subcommand

**Files:**
- Modify: `crates/byetex-cli/src/main.rs` (`Command` enum ~line 19; dispatch ~161; add `run_diagnose`)
- Test: `crates/byetex-cli/tests/diagnose.rs`

Reference the existing typst spawn in `byetex_output_compiles` (`main.rs:252`) and the
`typst_bin()` helper (`main.rs:244`) — reuse them; do not re-implement the spawn.

- [ ] **Step 1: Write the failing CLI integration test**

Create `crates/byetex-cli/tests/diagnose.rs`:

```rust
//! `byetex diagnose` writes a diagnostics.json mapping each typst error to the
//! originating LaTeX fragment + skill. Gated on `typst` being available.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn typst_available() -> bool {
    Command::new(std::env::var("BYETEX_TYPST_BIN").unwrap_or_else(|_| "typst".into()))
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_byetex"))
}

#[test]
fn diagnose_writes_diagnostics_json_with_mapped_error() {
    if !typst_available() {
        eprintln!("skipping: typst not on PATH");
        return;
    }
    let dir = std::env::temp_dir().join(format!("byetex-diag-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // A doc that converts but does NOT compile: a bare multi-letter word in math
    // that Typst reads as an unknown variable (after byetex emits it verbatim).
    let tex = dir.join("paper.tex");
    fs::write(&tex, "\\documentclass{article}\\begin{document}$x|arrival$\\end{document}\n").unwrap();

    let status = Command::new(bin())
        .arg("diagnose")
        .arg(&tex)
        .status()
        .unwrap();
    assert!(status.success(), "diagnose should exit 0 even when the paper has errors");

    let diag = fs::read_to_string(dir.join("paper.diagnostics.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&diag).unwrap();
    let arr = v.as_array().expect("diagnostics is a JSON array");
    assert!(!arr.is_empty(), "expected at least one mapped error, got: {diag}");
    let first = &arr[0];
    assert!(first.get("message").is_some());
    assert!(first.get("line").is_some());
    // src_fragment may be null if unmappable, but the field must be present.
    assert!(first.get("src_fragment").is_some());
    assert!(first.get("skill_name").is_some());

    let _ = fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p byetex-cli --test diagnose`
Expected: FAIL — `diagnose` is an unknown subcommand (clap error), so the binary exits non-zero
/ no `diagnostics.json` is written.

- [ ] **Step 3: Add the `Diagnose` subcommand variant**

In `crates/byetex-cli/src/main.rs`, in the `Command` enum (line ~19), add (mirror the
`Convert`/`AgentBrief` shape — it takes an input path, optional `--project`/`--out`):

```rust
    /// Convert, compile with typst, and write `<stem>.diagnostics.json` mapping
    /// each typst error back to its LaTeX source fragment + repair skill.
    Diagnose {
        /// Path to the input `.tex` (or project entry file).
        input: PathBuf,
        /// Convert as a project (copy assets, write main.typ) like `convert --project`.
        #[arg(long)]
        project: bool,
        /// Output directory (project mode) or output `.typ` path.
        #[arg(long)]
        out: Option<PathBuf>,
    },
```

In the dispatch `match cli.command { ... }` (line ~161), add:

```rust
        Command::Diagnose { input, project, out } => run_diagnose(input, project, out),
```

- [ ] **Step 4: Implement `run_diagnose`**

Add to `crates/byetex-cli/src/main.rs` (near `byetex_output_compiles`):

```rust
fn run_diagnose(input: PathBuf, project: bool, out: Option<PathBuf>) -> Result<()> {
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("read {}", input.display()))?;
    let base_dir = input.parent().map(|p| p.to_path_buf());

    // 1. Convert capturing the source map.
    let converted = byetex_core::convert_capturing_source_map(
        &source,
        &byetex_core::ConvertOptions {
            source_name: Some(input.display().to_string()),
            base_dir: base_dir.clone(),
        },
    );

    // 2. Write the .typ (flat next to input, or project main.typ).
    //    Reuse the existing flat/project writers if convenient; minimally:
    let typ_path = match (&out, project) {
        (Some(p), false) => p.clone(),
        _ => input.with_extension("typ"),
    };
    std::fs::write(&typ_path, &converted.typst)
        .with_context(|| format!("write {}", typ_path.display()))?;

    // 3. Shell typst compile (reuse the byetex_output_compiles spawn pattern).
    let pdf = typ_path.with_extension("pdf");
    let result = std::process::Command::new(typst_bin())
        .arg("compile").arg(&typ_path).arg(&pdf)
        .output()
        .with_context(|| format!("spawning `{}`", typst_bin()))?;
    let _ = std::fs::remove_file(&pdf);
    let stderr = String::from_utf8_lossy(&result.stderr);

    // 4. Parse errors and map each to source + skill.
    let typ_lines: Vec<&str> = converted.typst.lines().collect();
    let diagnostics: Vec<serde_json::Value> = byetex_core::parse_typst_errors(&stderr)
        .into_iter()
        .map(|e| {
            let line_text = typ_lines.get(e.line.saturating_sub(1)).copied().unwrap_or("");
            let span = byetex_core::resolve_error_line(&converted.source_map, line_text);
            let src_fragment = span.map(|(a, b)| source[a..b].to_string());
            // Skill: from the warning covering this source span, else null.
            let skill_name = span.and_then(|(a, b)| {
                converted.warnings.iter()
                    .find(|w| {
                        let r = &w.range;
                        // warnings carry byte ranges; pick one overlapping [a,b)
                        r.start_byte < b && r.end_byte > a
                    })
                    .and_then(|w| w.suggested_skill.clone())
            });
            serde_json::json!({
                "message": e.message,
                "line": e.line,
                "col": e.col,
                "src_fragment": src_fragment,
                "typ_region": line_text,
                "skill_name": skill_name,
            })
        })
        .collect();

    // 5. Write diagnostics.json next to the .typ.
    let diag_path = typ_path.with_extension("diagnostics.json");
    std::fs::write(&diag_path, serde_json::to_string_pretty(&diagnostics)?)
        .with_context(|| format!("write {}", diag_path.display()))?;
    eprintln!(
        "byetex diagnose: {} typst error(s) → {}",
        diagnostics.len(),
        diag_path.display()
    );
    Ok(())
}
```

Notes for the implementer:
- Confirm `Range` exposes `start_byte`/`end_byte` (it does — `warnings.rs::Range`). If the
  field names differ, adjust the overlap check.
- `serde_json` is already a dependency of `byetex-cli` (used for warnings.json).
- If `typst` is absent, `Command::output()` errors; wrap so diagnose still writes the `.typ`
  and an empty/`"compile skipped"` diagnostics file rather than failing hard (mirror
  `run_doctor`'s graceful-skip). Minimal: on spawn error, write `[]` and a stderr note.
- Project mode (`--project`): for the MVP, reuse the existing project writer
  (`run_project`'s materialization) to produce `main.typ`, then run steps 3–5 against
  `main.typ`. If wiring that is heavy, scope the first cut to flat mode and gate the test on
  flat (note the limitation in the subcommand help).

- [ ] **Step 5: Run the CLI test + full suite**

Run: `cargo test -p byetex-cli --test diagnose`
Expected: PASS (or a clean skip if typst is absent locally).
Run: `cargo test --workspace`
Expected: PASS.
Run: `cargo clippy -p byetex-core -p byetex-cli`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/byetex-cli/src/main.rs crates/byetex-cli/tests/diagnose.rs
git commit -m "feat(cli): byetex diagnose — compile + map typst errors to source + skill"
```

---

## Task 6: the init doc — `byetex-repair-loop` skill + for-agents.md

**Files:**
- Create: `skills/byetex-repair-loop.md`
- Modify: `docs/for-agents.md`
- Test: `crates/byetex-core/tests/default_skill.rs` (append a catalogue check)

- [ ] **Step 1: Write the failing test**

Append to `crates/byetex-core/tests/default_skill.rs`:

```rust
#[test]
fn repair_loop_skill_is_in_the_catalogue() {
    assert!(
        byetex_core::skills::read_skill("byetex-repair-loop").is_some(),
        "the byetex-repair-loop skill must be embedded"
    );
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p byetex-core --test default_skill repair_loop_skill_is_in_the_catalogue`
Expected: FAIL — the skill file doesn't exist yet (build.rs embeds `skills/*.md`; missing).

- [ ] **Step 3: Create the skill**

Create `skills/byetex-repair-loop.md` (frontmatter `name`/`description` are parsed by
`build.rs`):

```markdown
---
name: byetex-repair-loop
description: The CLI repair loop — use `byetex diagnose` to compile the generated Typst, map each error to its LaTeX fragment + skill, fix the .typ, and verify with `typst compile`.
---

# byetex repair loop

When a converted paper doesn't compile, repair the `.typ` one error at a time.

## Loop

1. **Diagnose once.** `byetex diagnose paper.tex` writes:
   - `paper.typ` — the generated Typst.
   - `paper.diagnostics.json` — an array of `{message, line, col, src_fragment, typ_region, skill_name}`, one per typst error.
2. **For each diagnostic:**
   - Read `src_fragment` (the LaTeX that produced the failing region) and `typ_region`
     (the offending `.typ` line).
   - If `skill_name` is set, read it: `byetex skills read <skill_name>`.
   - Apply the **smallest** local edit to `paper.typ` that fixes that error. Preserve what
     already works.
3. **Verify.** Run `typst compile paper.typ`. If it still reports errors, fix the next one
   and re-run. Repeat until it compiles.

## Rules

- **Do NOT re-run `byetex diagnose` after editing** — it re-converts from source and
  overwrites your edits to `paper.typ`. Use `typst compile paper.typ` to iterate; only
  re-run `diagnose` to start over from the LaTeX source.
- `src_fragment` / `skill_name` are `null` when an error can't be mapped (e.g. it's in the
  preamble or a region you already edited) — fall back to the raw typst `message`.
- Fix the smallest, most local thing per error; don't rewrite whole blocks.
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p byetex-core --test default_skill repair_loop_skill_is_in_the_catalogue`
Expected: PASS (build.rs picks up the new `skills/*.md` and embeds it).

- [ ] **Step 5: Add the loop diagram to for-agents.md**

In `docs/for-agents.md`, add a "Repair loop" section with this diagram and a one-line pointer
to `byetex skills read byetex-repair-loop`:

```text
byetex diagnose paper.tex
  → paper.typ + paper.diagnostics.json  (per error: src_fragment, typ_region, skill_name)
  → for each error: read skill, edit paper.typ
  → typst compile paper.typ  ──(errors?)──┐
        ▲                                  │ loop until clean
        └──────────────────────────────────┘
```

- [ ] **Step 6: Run the full suite**

Run: `cargo test --workspace`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add skills/byetex-repair-loop.md docs/for-agents.md \
        crates/byetex-core/tests/default_skill.rs
git commit -m "docs(skills): byetex-repair-loop skill + for-agents loop diagram"
```

---

## Final verification (after all tasks)

- [ ] `cargo test --workspace` — green.
- [ ] `cargo clippy -p byetex-core -p byetex-cli` — clean.
- [ ] Goldens/snapshots unchanged except the intentional `suggested_skill` `null`→name flips
  from Task 1.
- [ ] Manual smoke: `byetex diagnose corpus/2605.31561/source/main.tex` (a known compile
  failure) → `paper.diagnostics.json` maps at least one error to a plausible LaTeX fragment
  and a readable skill; `byetex skills read <skill_name>` prints it; edit `paper.typ`; `typst
  compile paper.typ` confirms the fix.
- [ ] `scripts/acceptance.sh` still green (tooling-only change; compile-rate unchanged).
- [ ] Open the PR for `design/ai-fallback-repair`.
