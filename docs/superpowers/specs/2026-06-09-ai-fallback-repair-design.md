# AI fallback / scoped fragment repair — design

**Status:** draft for review (brainstorming output, pre-plan)
**Date:** 2026-06-09 (rev. 2026-06-10 — CLI-primary, MCP deferred)
**Goal:** Close the agent-repair loop on the **CLI path**: when byetex output doesn't
compile, give an external AI agent everything it needs to fix it *at the fragment level* —
the compile result, each typst error mapped back to the originating LaTeX source span, and
the matching repair skill. byetex stays a pure tool; the agent (Claude Code, with a shell)
drives the loop using the CLI + skills + an init doc.

---

## Context — why this, and what already exists

ByeTex converts LaTeX → Typst with "convert what you can, warn on the rest." Phase 3's
second half is the **scoped AI fragment repair** loop, deferred until after the emit.rs
refactor. The *scaffolding* exists; the *loop* does not.

**Already built (the agent can already use most of this from a shell):**
- CLI: `byetex convert [--project]` → writes `.typ`, `<stem>.warnings.json`, and a compact
  `agent_brief.md`; `byetex agent-brief` additionally shells `typst compile` and embeds the
  status; `byetex skills list` / `byetex skills read <name>`.
- 6 skill markdown guides under `skills/`, embedded at build time
  (`crates/byetex-core/src/skills.rs`, `build.rs`).
- Structured warnings with **source byte-ranges** and `suggested_skill: Option<String>`
  (`crates/byetex-core/src/warnings.rs`).
- A `byetex-mcp` server mirroring `convert`/skills over JSON-RPC.

**Key reframing (this revision):** the agent has a shell, so the **CLI + skills + a good
init doc are the product**. The MCP is just a second transport over capabilities the CLI
already exposes — **not important for this effort**, so it's deferred. The *only* genuinely
new capability is mapping a typst compile error back to the LaTeX source fragment; that
goes in core + a `byetex diagnose` CLI subcommand.

**The gaps this design fills:**
1. No **source map**: a typst error at a `.typ` location can't be traced to the LaTeX
   fragment that produced it — so "scoped" repair has no targeting.
2. No single CLI step that **compiles + maps errors + names skills** in one shot.
3. `suggested_skill` is **barely wired** (4 sites, all → `unsupported-environment`); most
   categories leave it `None` despite matching skills existing.
4. No **init doc / skill** telling an agent the exact repair loop.

**Decisions locked in brainstorming:**
- **Tools-only, CLI-primary.** byetex stays a pure tool (no LLM, no network, no agent loop
  inside it). The agent drives the loop from a shell via the CLI + skills + init doc.
- **byetex wraps `typst`.** The CLI shells `typst compile` (already done by
  `byetex agent-brief`); `byetex diagnose` reuses that path. `typst` on PATH is an assumed
  dev dependency.
- **Node-level source map via `emit_node`.** Single-point instrumentation in the dispatcher.
- **`byetex diagnose` is the agent's main entry point.** MCP is deferred (can mirror later).

---

## Architecture & data flow

byetex is a pure CLI tool; Claude Code drives the loop from a shell:

```
 source.tex ─► byetex diagnose ─►  paper.typ
  (or project)        │            paper.diagnostics.json   ← per-error: {message, line,
                      │            agent_brief.md (compile-aware)  src_fragment, typ_region,
                      │                                            skill_name}
                      ▼
            agent reads diagnostics + reads the named skills (`byetex skills read <name>`),
            applies the smallest local edits to paper.typ
                      │
                      ▼
            agent runs `typst compile paper.typ`  ──► errors? ──┐
                      ▲                                          │ loop until clean
                      └──────────────────────────────────────────┘
```

- **`byetex diagnose <source>`** — the rich first call. Converts the source *with the
  source map captured*, writes `paper.typ`, shells `typst compile`, parses each error,
  resolves its `.typ` offset through the map to the originating LaTeX fragment, attaches the
  matching skill name, and writes `paper.diagnostics.json` + a compile-aware
  `agent_brief.md`.
- **Iteration is the agent's own `typst compile paper.typ`** — it has a shell and needs only
  the ground-truth error list to keep editing. No byetex round-trip required per turn.
- **Skill bodies** come from the existing `byetex skills read <name>` (the diagnostics name
  the skill; the agent reads it).

**Drift rule:** `byetex diagnose` re-converts from source and **overwrites `paper.typ`** —
so it's the *one-shot* initial analysis. After the agent edits `paper.typ`, it must NOT
re-run `diagnose` (that clobbers the edits); it verifies with `typst compile`. Re-run
`diagnose` only to restart from source. The source map is therefore authoritative for the
first analysis; `typst compile` is ground truth during iteration.

---

## Components

### 1. Source map (`byetex-core`) — the only new core capability

**Content-anchored** (not byte-offset). `finish()` relocates the body and
`post_process_typography` rewrites the whole output char-by-char *after* emission, so any
byte offsets captured during `emit_node` would drift. Instead, record each node's **output
text** alongside its source span, and at diagnose time match a typst error's *line text* to
the node that produced it. Matching on content is immune to the post-emit byte shifts.

```rust
pub struct NodeOutput {
    pub src: (usize, usize),  // byte range in the LaTeX source
    pub output: String,       // the .typ text this node produced (pre-post-process)
}
pub source_map: Vec<NodeOutput>,   // new ConvertOutput field; empty unless requested
```

**Single-point instrumentation:** rename the dispatcher body to `emit_node_inner`; make
`emit_node` a thin wrapper that captures each node's output slice:

```rust
fn emit_node(&mut self, node: Node<'_>) -> usize {
    if !self.record_source_map { return self.emit_node_inner(node); }
    let out_start = self.out.len();
    let src = (node.start_byte(), node.end_byte());
    let r = self.emit_node_inner(node);
    if self.out.len() > out_start {
        self.source_map.push(NodeOutput {
            src,
            output: self.out[out_start..].to_string(),
        });
    }
    r
}
```

- **Nesting:** a parent's `output` contains its children's. Lookup returns the node with the
  **shortest** `output` that still contains the query line → most specific (leaf-most)
  source span.
- **Gated:** `record_source_map` (new `Emitter` flag, default off) → normal `convert` is
  zero-overhead and byte-identical (goldens unaffected). `diagnose` turns it on.
- **Sub-buffers need no special-casing.** `with_sub_buffer` / `render_in_sub_emitter`
  emit into a temporary buffer; entries recorded there hold valid output *text*. If that
  text is spliced verbatim into the final `.typ`, it matches (finer granularity for free —
  cell-level rather than table-level); if it's transformed or discarded before splicing, the
  entry simply never matches any error line (inert). The outer node also records a coarse
  entry (whole table → whole `tabular` source span), so a line always resolves to *something*
  — finest match wins. (Byte-offset mapping would have needed save/restore here; content
  matching does not.)
- **No offset bookkeeping:** matching is on text, so `finish()`'s relocation and
  `post_process_typography` need no special handling — the recorded `output` is the
  pre-post-process slice, and the small per-line edits post-processing makes still leave the
  bulk of the line matchable.
- **Memory:** recording every node's output (gated, one-shot during `diagnose`) is O(total
  output × depth) of cloned strings — a few MB for a typical paper, acceptable for a one-shot
  call. Bounding it (offsets + snapshot, or skipping huge nodes) is a deferred optimization.

**Lookup:** `fn resolve_error_line(map: &[NodeOutput], typ_line: &str) -> Option<(usize,usize)>`
— normalize whitespace; among nodes whose `output` contains the normalized `typ_line`, return
the `src` of the one with the **shortest** `output`; if none contains the full line, fall
back to the node whose `output` contains the line's longest non-whitespace token; else
`None`. Pure, unit-testable, in core.

### 2. typst diagnostic parser (`byetex-core::typst_diag`)

Pure parser (no process spawning) so it's unit-testable: given `typst compile` stderr text,
return:

```rust
pub struct TypstError { pub message: String, pub line: usize, pub col: usize }
pub fn parse_typst_errors(stderr: &str) -> Vec<TypstError>;
```

Scans for `error: <msg>` + the `┌─ <file>:<line>:<col>` location line (1-based). The
content-anchored map resolves source from the line *text* (fetched by line number), so no
byte resolution is needed here.

### 3. `byetex diagnose` CLI subcommand (`byetex-cli`)

Reuses the existing `agent-brief` compile path. Steps:
1. Convert source with `record_source_map = true` → `.typ` + `source_map` + warnings.
2. Write `paper.typ`.
3. Shell `typst compile paper.typ <tmp>` (the existing invocation), capture stderr.
4. `parse_typst_errors(stderr)`; for each error: fetch the `.typ` line text at `error.line`;
   `resolve_error_line(source_map, line_text)` → source span; slice the `.tex` for
   `src_fragment` and use the line text for `typ_region`; pick the skill from the byetex
   warning covering that source span, else `default_skill_for(category)`.
5. Write `paper.diagnostics.json` (array of `{message, line, col, src_fragment, typ_region,
   skill_name}`) and a compile-aware `agent_brief.md` that references it.

Flags: `--project` (project mode, like convert); `--out`. Same fast/quiet ergonomics as the
other subcommands.

### 4. `suggested_skill` wiring (`byetex-core`)

Central category → skill mapping so every warning points at a guide:

```rust
fn default_skill_for(cat: &Category) -> Option<&'static str> {
    match cat {
        Category::UnsupportedEnvironment{..} => Some("byetex-unsupported-environment"),
        Category::Tikz                       => Some("byetex-tikz-to-typst"),
        Category::CustomMacro{..}            => Some("byetex-custom-macros"),
        Category::ParseError{..}             => Some("byetex-parse-error"),
        Category::AmbiguousMath{..}          => Some("byetex-using-warnings-json"),
        Category::UnsupportedCommand{..}     => Some("byetex-using-warnings-json"),
        Category::NeedsManualReview{..}      => Some("byetex-using-warnings-json"),
        _ => None,
    }
}
```

Applied at warning finalization to fill `suggested_skill` when an emit site didn't set one.
Bibliography warnings (raised as `NeedsManualReview`) get `byetex-bibliography` set
explicitly at their emit sites (overriding the category default).

### 5. Agent guidance — the init doc (`skills/` + `docs/for-agents.md`)

A new short skill `byetex-repair-loop.md` documents the exact loop: run `byetex diagnose`,
read `paper.diagnostics.json`, for each error read `src_fragment` + `byetex skills read
<skill_name>`, apply the smallest edit to that region in `paper.typ`, run
`typst compile paper.typ` to verify, repeat; re-run `diagnose` only to restart. The loop
diagram is added to `docs/for-agents.md`. (This skill + doc is what makes "CLI + skills +
init doc" sufficient without an MCP.)

---

## Error handling

- **`typst` not on PATH:** `diagnose` still converts and writes `.typ` + warnings, and the
  diagnostics report `compile: skipped (typst not found)` — never panic.
- **Clean compile:** `diagnostics.json` is `[]`, brief says `✅ compiles` — agent is done.
- **Unmappable error** (preamble or, on a re-run, an agent-edited region): emit the error
  with `src_fragment: null`; still attach a best-effort skill via the nearest warning, else
  `null`. The raw typst message is always included.
- **Compile timeout** (pathological input): bounded timeout on the `typst` child → report
  `compile: timed out`.
- **Many errors:** all reported (typst emits many per run); the agent fixes smallest-first
  and re-compiles.

---

## Testing

- **Source map (core, unit):** convert a 2-construct doc with `record_source_map`; assert a
  line of construct A's output resolves (via `resolve_error_line`) to A's source span and is
  the *shortest-output* (most specific) match; a sub-buffer-coarseness test (a line inside a
  table resolves to the whole `tabular` source span, no garbage entries); a gated-off test
  (default convert produces an empty `source_map` and byte-identical output → goldens
  unaffected).
- **typst diagnostic parser (core, unit):** canned `typst` stderr → structured
  `Vec<{message,line,col}>` (multi-error input).
- **`default_skill_for` (unit):** each category → expected skill; assert each returned name
  exists in the catalogue (`read_skill` is `Some`).
- **CLI integration (`byetex-cli/tests`):** `byetex diagnose` on a fixture with one injected
  error → `diagnostics.json` maps the error to the correct source fragment + skill name;
  on a clean fixture → `diagnostics.json` is `[]`. Gated/skipped when `typst` is absent
  (mirror the existing brief tests' approach).
- **End-to-end (manual / under acceptance harness, which has typst):** `byetex diagnose` a
  corpus paper that fails → errors map to plausible source fragments + skills.

---

## MVP scope (this spec) vs deferred

**In scope (Phase 1):**
1. Source map infra: `emit_node` wrapper + gating + sub-buffer save/restore + preamble
   shift + `resolve_typ_offset` + `ConvertOutput.source_map`.
2. `byetex-core::typst_diag` parser (stderr → structured errors → byte offsets).
3. `byetex diagnose` CLI subcommand (convert+compile+map → `diagnostics.json` +
   compile-aware brief).
4. `default_skill_for` central wiring across all categories (+ explicit bibliography).
5. `byetex-repair-loop` skill + `docs/for-agents.md` loop diagram (the init doc).

**Deferred (not now):**
- **The MCP `diagnose`/`compile` tools** — the MCP is deferred entirely; it can mirror the
  CLI capability later (the core logic is shared, so it's cheap to add if ever wanted).
- Full byte-level source map / fine-grained sub-buffer mapping.
- Implementing `convert_fragment`'s `context_hint` (true fragment re-conversion).
- byetex applying patches itself / any in-tool LLM (explicitly rejected — tools-only).

---

## Out of scope

- The agent's editing strategy / prompt engineering (lives in the agent, not byetex).
- Changing conversion to "fix" papers (this is a *repair-assist* surface, not more converter
  coverage).
- Re-platforming or touching `class_map` / `project` mode beyond reading them.

---

## Verification (end to end)

1. `cargo test --workspace` — new unit/CLI tests green; existing suite unaffected
   (source-map capture gated off by default → snapshots/goldens byte-identical).
2. `cargo clippy -p byetex-core -p byetex-cli` clean.
3. Manual: `byetex diagnose corpus/<id>/source/main.tex --project --out /tmp/x`; confirm
   `diagnostics.json` maps a real typst error to the right LaTeX fragment + a readable
   skill; edit `paper.typ`; `typst compile`; confirm the fix.
4. Acceptance gate (`scripts/acceptance.sh`) still green — this adds tooling, not conversion
   changes, so compile-rate is unchanged.
