# AI fallback / scoped fragment repair — design

**Status:** draft for review (brainstorming output, pre-plan)
**Date:** 2026-06-09
**Goal:** Close the agent-repair loop — when byetex output doesn't compile, give an
external AI agent everything it needs to fix it *at the fragment level*: a compile
result, each error mapped back to the originating LaTeX source span, and the matching
repair skill. byetex stays a pure tool; the agent drives the loop.

---

## Context — why this, and what already exists

ByeTex converts LaTeX → Typst with "convert what you can, warn on the rest." Phase 3's
second half is the **scoped AI fragment repair** loop, deliberately deferred until after
the emit.rs refactor. The *scaffolding* exists; the *loop* does not.

**Already built:**
- `byetex-mcp` server (`byetex serve`, stdio JSON-RPC, on by default): tools `convert`,
  `convert_file`, `convert_fragment`, `convert_project`, `list_skills`, `read_skill`
  (`crates/byetex-mcp/src/lib.rs`).
- 6 skill markdown guides under `skills/`, embedded at build time
  (`crates/byetex-core/src/skills.rs`, `build.rs`); accessible via CLI
  (`byetex skills read <name>`) and MCP.
- Structured warnings with `suggested_skill: Option<String>`
  (`crates/byetex-core/src/warnings.rs`).
- `agent_brief.md` per-paper sidecar (CLI, on by default) — task text, warnings
  histogram, file paths, and (only via `byetex agent-brief`) a `typst compile` status.

**The gaps this design fills:**
1. Nothing returns a **compile result** through the tooling (typst is only shelled out
   into the brief's text by the CLI; the MCP can't validate output).
2. No **source map**: a typst error at a `.typ` location can't be traced to the LaTeX
   fragment that produced it — so "scoped" repair has no targeting.
3. `suggested_skill` is **barely wired** (4 sites, all → `unsupported-environment`);
   `parse_error`/`tikz`/`custom_macro`/`bibliography`/`ambiguous_math`/`unsupported_command`
   leave it `None` despite matching skills existing.

**Decisions locked in brainstorming (these frame the whole design):**
- **Tools-only.** byetex stays a pure tool (no LLM, no network, no agent loop inside it).
  An external agent (Claude Code / any MCP client) drives iteration. Honors the core
  invariant: `byetex-core` has no FS/CLI/network deps; `byetex-mcp` is the agent layer.
- **byetex-mcp wraps `typst`.** A tool shells out to `typst compile` and returns
  structured errors. (`typst` on PATH is already an assumed dev dependency.)
- **Node-level source map via `emit_node`.** Single-point instrumentation in the
  dispatcher, not the ~hundreds of emit sites.
- **One high-level `diagnose` tool** is the agent's main entry point.

---

## Architecture & data flow

byetex is a pure tool; Claude Code drives the loop:

```
              ┌──────────────── external agent (Claude Code) ─────────────────┐
              │                                                                 │
 source.tex ─► diagnose(source) ─►  paper.typ  +  [ {error, src_fragment,        │
  (or project)       │                                typ_region, skill}, … ]    │
                     │                                                           ▼
                     │                                            agent applies smallest
                     │                                            local edits to paper.typ
                     │                                                           │
                     └────────────  compile(paper.typ) ◄──────────────────────────┘
                                          │  loop until clean / agent's budget
                                          ▼
                                    { ok } | { remaining structured errors }
```

- **`diagnose(source)`** — the rich first call. byetex converts the source *in-memory*
  (capturing the source map), writes `paper.typ`, shells `typst compile`, parses each
  error, resolves its `.typ` offset through the map to the originating LaTeX fragment,
  and attaches the matching skill body. One structured bundle.
- **`compile(typ_path)`** — the cheap iteration call. Runs `typst compile`, returns
  structured errors only. The agent edits `paper.typ` and re-calls this for ground truth.
- byetex never edits and never calls an LLM. It converts, compiles, maps, explains. The
  agent edits and decides when to stop.

**Drift rule (important):** the source map is precise for the *byetex-generated* `.typ`.
Once the agent edits, its own regions aren't in the map. So `diagnose`'s source→fragment
mapping is authoritative on the **first** pass (understand the failures); `compile`
provides ground-truth feedback during iteration. byetex never re-converts mid-loop — that
would clobber the agent's edits.

---

## Components

### 1. Source map (`byetex-core`)

**What:** an interval map from `.typ` byte ranges → originating `.tex` byte ranges, built
during conversion, exposed on `ConvertOutput`.

```rust
pub struct SpanMapping {
    pub typ: (usize, usize),  // byte range in the generated .typ
    pub src: (usize, usize),  // byte range in the LaTeX source
}
// new field on ConvertOutput:
pub source_map: Vec<SpanMapping>,   // empty unless capture was requested
```

**How (single-point instrumentation):** rename the existing dispatcher body to
`emit_node_inner`; make `emit_node` a thin wrapper that brackets each node's output:

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

- **Nested intervals:** a parent node's interval contains its children's. Lookup returns
  the **smallest** interval containing a given `.typ` offset → the most specific source
  span.
- **Gated:** `record_source_map` (a new `Emitter` flag, default off) keeps normal
  `convert` zero-overhead. `diagnose` turns it on.
- **Sub-buffers:** `with_sub_buffer` / `render_in_sub_emitter` swap `self.out` for a fresh
  buffer; offsets recorded there would be relative to the wrong buffer. **MVP rule:**
  disable `record_source_map` for the duration of a sub-buffer (save/restore the flag).
  The *outer* node's wrapper still records a correct **coarse** interval (the whole
  table / math block → the whole `tabular` / equation source span), because the splice
  into `self.out` happens inside the outer `emit_node` call. Fine-grained sub-node mapping
  is deferred (see Out of scope).
- **Preamble shift:** the body is emitted first (offsets relative to the body), then
  `finish()` prepends the neutral preamble. After prepend, add `preamble.len()` to every
  `SpanMapping.typ` range in one pass.

**Lookup:** `fn resolve_typ_offset(map: &[SpanMapping], off: usize) -> Option<(usize, usize)>`
returns the smallest `src` span whose `typ` range contains `off`. Lives in core (pure,
unit-testable).

### 2. `compile` tool (`byetex-mcp`)

Shell out to `typst compile <path> <tmp.pdf>` (or `--format pdf` to a temp), capture
stderr, parse into structured errors:

```rust
pub struct TypstError {
    pub message: String,
    pub typ_path: String,
    pub line: usize, pub col: usize,   // 1-based, as typst reports
    pub typ_byte: usize,               // resolved from line:col against the .typ bytes
}
```

Parser: scan stderr for `error: <msg>` followed by `┌─ <file>:<line>:<col>` (typst's
diagnostic format). A small, well-tested string parser in `byetex-mcp` (or a
`byetex-core::typst_diag` helper so it's unit-testable without the binary). Returns
`{ ok: bool, errors: Vec<TypstError> }`.

### 3. `diagnose` tool (`byetex-mcp`)

The composition. Input: a source `.tex` path (or project main) + optional `out_dir`.

1. Convert with `record_source_map = true` → `.typ` + `source_map` + warnings.
2. Write `paper.typ` (so the agent has a file to edit).
3. `compile(paper.typ)` → structured typst errors.
4. For each error: `resolve_typ_offset(source_map, error.typ_byte)` → source span; slice
   the `.tex` for `src_fragment` and the `.typ` for `typ_region`; find the byetex warning
   covering that source span (if any) and its skill; else fall back to a category→skill
   default.
5. Return one bundle per error: `{ message, line, col, src_fragment, typ_region,
   skill: {name, body} | null }`, plus the overall `{ ok, typ_path, warnings_summary }`.

### 4. `suggested_skill` wiring (`byetex-core`)

Central mapping so every warning category points at a guide:

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

Applied at warning-finalization (fill `suggested_skill` from the category when an emit
site didn't set one explicitly). Keeps the warning→skill links honest without editing
every emit site. A new `byetex-bibliography` association is wired where bib warnings are
raised.

### 5. Agent guidance (`skills/` + `docs/for-agents.md`)

A short new skill `byetex-repair-loop.md` documents the exact loop: call `diagnose`, read
each error's `src_fragment` + `skill`, apply the smallest edit to `typ_region` in
`paper.typ`, call `compile` to verify, repeat. `docs/for-agents.md` gets the loop diagram.

---

## Error handling

- **`typst` not on PATH:** `compile`/`diagnose` return a structured `{ ok:false, error:
  "typst not found on PATH" }` — never panic.
- **Clean compile:** `diagnose` returns `errors: []`, `ok:true` — the agent is done.
- **Unmappable error** (location in preamble or an agent-edited region): return the error
  with `src_fragment: null`; still attach a best-effort skill via the nearest warning, or
  `null`. The agent can still act on the raw typst message.
- **Compile timeout** (pathological input): bounded timeout on the `typst` child; on
  timeout return `{ ok:false, error:"typst compile timed out" }`.
- **Multiple errors:** all returned (typst reports many per run); the agent fixes
  smallest-first and re-compiles.

---

## Testing

- **Source map (core, unit):** convert a 2-construct doc; assert a `.typ` offset inside
  construct A's output resolves to A's source span and is the *smallest* containing
  interval. A preamble-shift test (offset after `finish` still resolves). A
  sub-buffer-gating test (a table maps coarsely to the whole `tabular` span, no garbage
  entries).
- **typst diagnostic parser (unit):** feed canned `typst` stderr; assert structured
  `{message,line,col}` and line:col→byte resolution against a known `.typ`.
- **`default_skill_for` (unit):** each category → expected skill; round-trip that the
  skill name exists in the catalogue.
- **MCP integration:** `diagnose` on a fixture with one injected error → returns the error
  mapped to the correct source fragment + skill; `compile` on a clean `.typ` → `ok:true`;
  `compile` with no `typst` → graceful error (skip/ignore in CI if typst absent).
- **End-to-end (manual / gated):** `diagnose` a corpus paper that fails to compile →
  errors map to plausible source fragments. Not in the default `cargo test` (needs typst);
  runs under the existing acceptance harness which already has typst.

---

## MVP scope (this spec) vs deferred

**In scope (Phase 1):**
1. Source map infra: `emit_node` wrapper + gating + sub-buffer save/restore + preamble
   shift + `resolve_typ_offset` + `ConvertOutput.source_map`.
2. typst diagnostic parser + line:col→byte resolution (`byetex-core::typst_diag`).
3. `compile` and `diagnose` MCP tools.
4. `default_skill_for` central wiring across all categories (+ bibliography).
5. `byetex-repair-loop` skill + `for-agents.md` loop diagram.

**Deferred (not now):**
- Full byte-level source map / fine-grained sub-buffer mapping.
- Implementing `convert_fragment`'s `context_hint` (true fragment re-conversion).
- byetex applying patches itself / any in-tool LLM (explicitly rejected — tools-only).
- A `sourcemap.json` sidecar from the CLI (the map is used in-process by `diagnose`;
  a sidecar can come later if a CLI loop is wanted).

---

## Out of scope

- The agent's editing strategy / prompt engineering (lives in the agent, not byetex).
- Changing the conversion behavior to "fix" papers (this is a *repair-assist* surface, not
  more converter coverage).
- Re-platforming or touching `class_map` / `project` mode beyond reading them.

---

## Verification (end to end)

1. `cargo test --workspace` — new unit tests green; existing suite unaffected (source-map
   capture is gated off by default, so snapshots/goldens are byte-identical).
2. `cargo clippy -p byetex-core -p byetex-mcp` clean.
3. Manual: `byetex serve`; from an MCP client call `diagnose` on a corpus paper with a
   known typst error; confirm the returned bundle maps the error to the right LaTeX
   fragment and a readable skill; edit `paper.typ`; call `compile`; confirm it reports the
   fix. (Or drive the same via the existing `mcp_smoke.rs` style integration test.)
4. Acceptance gate (`scripts/acceptance.sh`) still green — this adds tooling, not
   conversion changes, so compile-rate is unchanged.
