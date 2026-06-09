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

Interval map from `.typ` byte ranges → originating `.tex` byte ranges, built during
conversion, exposed on `ConvertOutput`.

```rust
pub struct SpanMapping {
    pub typ: (usize, usize),  // byte range in the generated .typ
    pub src: (usize, usize),  // byte range in the LaTeX source
}
pub source_map: Vec<SpanMapping>,   // new ConvertOutput field; empty unless requested
```

**Single-point instrumentation:** rename the dispatcher body to `emit_node_inner`; make
`emit_node` a thin wrapper that brackets each node's output:

```rust
fn emit_node(&mut self, node: Node<'_>) -> usize {
    if !self.record_source_map { return self.emit_node_inner(node); }
    let out_start = self.out.len();
    let src = (node.start_byte(), node.end_byte());
    let r = self.emit_node_inner(node);
    let out_end = self.out.len();
    if out_end > out_start {
        self.source_map.push(SpanMapping { typ: (out_start, out_end), src });
    }
    r
}
```

- **Nested intervals:** a parent's interval contains its children's; lookup returns the
  **smallest** interval containing a `.typ` offset → most specific source span.
- **Gated:** `record_source_map` (new `Emitter` flag, default off) → normal `convert` is
  zero-overhead and byte-identical (goldens unaffected). `diagnose` turns it on.
- **Sub-buffers:** `with_sub_buffer` / `render_in_sub_emitter` swap `self.out`; offsets
  recorded there would be relative to the wrong buffer. **MVP rule:** save/restore-disable
  `record_source_map` for the duration of a sub-buffer. The *outer* node still records a
  correct **coarse** interval (whole table / equation → whole `tabular` / math source span),
  because the splice into `self.out` happens inside the outer `emit_node`. Fine-grained
  sub-node mapping is deferred.
- **Preamble shift:** body is emitted first (offsets relative to the body), then `finish()`
  prepends the neutral preamble; add `preamble.len()` to every `typ` range in one pass.

**Lookup:** `fn resolve_typ_offset(map, off) -> Option<(usize,usize)>` — smallest `src` span
whose `typ` range contains `off`. Pure, unit-testable, in core.

### 2. typst diagnostic parser (`byetex-core::typst_diag`)

Pure parser (no process spawning) so it's unit-testable: given `typst compile` stderr text
and the `.typ` content, return:

```rust
pub struct TypstError { pub message: String, pub line: usize, pub col: usize, pub typ_byte: usize }
pub fn parse_typst_errors(stderr: &str, typ_src: &str) -> Vec<TypstError>;
```

Scans for `error: <msg>` + the `┌─ <file>:<line>:<col>` location line; resolves `line:col`
→ `typ_byte` against `typ_src`.

### 3. `byetex diagnose` CLI subcommand (`byetex-cli`)

Reuses the existing `agent-brief` compile path. Steps:
1. Convert source with `record_source_map = true` → `.typ` + `source_map` + warnings.
2. Write `paper.typ`.
3. Shell `typst compile paper.typ <tmp>` (the existing invocation), capture stderr.
4. `parse_typst_errors(stderr, typ_src)`; for each: `resolve_typ_offset` → source span;
   slice `.tex`/`.typ` for `src_fragment`/`typ_region`; pick the skill from the byetex
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

- **Source map (core, unit):** convert a 2-construct doc; assert a `.typ` offset inside
  construct A's output resolves to A's source span and is the *smallest* containing
  interval; a preamble-shift test; a sub-buffer-gating test (a table maps coarsely to the
  whole `tabular` span, no garbage entries); a gated-off test (default convert produces an
  empty `source_map` and byte-identical output → goldens unaffected).
- **typst diagnostic parser (core, unit):** canned `typst` stderr → structured
  `{message,line,col}` + correct `line:col`→byte resolution against a known `.typ`.
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
