# Architecture

ByeTex converts LaTeX source into [Typst](https://typst.app). It targets the
academic-paper subset of LaTeX and, for anything outside that subset, degrades
gracefully — emitting a structured `warnings.json` sidecar (and a per-document
`agent_brief.md`) instead of failing — so a human or an LLM can finish the job.

This document is the overview; for detail beyond it, see the deeper docs:

- [`getting-started.md`](getting-started.md) — install and first conversions.
- [`conversion-logic.md`](conversion-logic.md) — how the emitter behaves, in prose.
- [`for-agents.md`](for-agents.md) — the contract for AI agents.
- [`emit-refactor-insights.md`](emit-refactor-insights.md) — internals of `emit.rs`.
- [`tectonic-integration-analysis.md`](tectonic-integration-analysis.md) — the `doctor` oracle.

It records only slow-changing facts. It names files and types but does not link
to them or cite line numbers — use symbol search. Review it periodically, not
on every change.

## Bird's Eye View

LaTeX is an unbounded macro language with no fixed grammar; Typst is a small,
structured markup language. A faithful 1:1 translation is impossible in general,
so ByeTex makes the problem tractable with two ideas: **convert what it can and
warn on the rest** (never guess, never hard-fail), and **always emit a neutral,
self-contained preamble** rather than trying to reproduce a document class.

The conversion is a single forward walk of a tree-sitter syntax tree — there is
no intermediate IR. The `Emitter` writes Typst directly into a buffer and
accumulates warnings, assets, and metadata as it goes.

```text
LaTeX source
  → parser.rs                 (tree-sitter LaTeX grammar, via FFI)  : concrete syntax tree
  → Emitter::prepass_collect  harvest document class, macros, \newif flags,
                              title/authors/abstract, bib keys, labels
  → Emitter::emit_root        walk the tree: expand macros, inline \input,
    / emit_node               translate math, emit warnings inline
  → Emitter::finish           prepend the neutral preamble
                              → ConvertOutput { typst, warnings, asset_refs, class_metadata }

Project mode wraps the above (project.rs):
  plan_project        scan the \input tree, harvest every macro & label up front,
                      convert with them pre-seeded → ProjectPlan { body + asset list }
  materialize_project copy assets, write typst.toml + sidecars   (called by the CLI)

The CLI and MCP layers own all filesystem I/O and process spawning (typst, tectonic).
```

## Code Map

A Cargo workspace of three crates: a pure conversion library, a CLI binary, and
an MCP server. The CLI depends on the core (and optionally the MCP crate); the
MCP crate depends on the core.

### `crates/byetex-core` — the conversion library

Where essentially all the logic lives.

> Architecture Invariant: the core has no filesystem, CLI, or MCP dependencies
> — its only dependencies are the parser, `serde`, and `anyhow`. Every entry
> point is a pure function over strings, testable without touching disk.

`lib.rs` — the public API: `convert(source, &ConvertOptions) -> ConvertOutput`,
plus an internal `convert_with_macros` the project layer uses to pre-seed macros
and labels.

> API Boundary: `convert` is the one stable entry point; both the CLI and the
> MCP server go through it.

`parser.rs` — a thin wrapper over the vendored tree-sitter LaTeX grammar.
`parse(source) -> Tree`; the tree's `has_error` flag drives parse-error warnings.

> Architecture Invariant: ByeTex does not write its own LaTeX parser — it
> reuses the same tree-sitter grammar that powers editor highlighting.

`emit.rs` — the `Emitter` state machine; the largest and most central module:
the two-pass `prepass_collect` → `emit_root`/`emit_node` dispatch over the
node kinds, then `finish`. See `emit-refactor-insights.md` for its internals.

> Architecture Invariant: math is hand-rolled (the syntax tree is translated
> through a manual symbol table), not delegated to MiTeX, KaTeX, or any engine.
> This is deliberate — MiTeX was evaluated and rejected.

> Architecture Invariant: every document is rendered with a single
> self-generated neutral preamble; ByeTex never binds a Typst Universe template.

`emit/boundary.rs`, `emit/escape.rs` — math-identifier spacing and output
escaping, split out of the emitter.

`document.rs` — `DocumentMetadata` (title, authors, abstract, …).

`class_map.rs` — `DocClass` detection. Used only to drive class-specific
author-block parsing and retain layout hints — never to select a template.

`package_macros.rs` — bundled macro seed tables (KaTeX builtins, `physics`,
`bm`, …). All seeds yield to user `\newcommand`s.

`bib.rs` — a BibTeX preprocessor that rewrites real-world `.bib` quirks into a
form Typst's strict parser accepts.

`project.rs` — `plan_project`, `ProjectPlan`, `materialize_project`:
multi-file project orchestration over `convert`.

> Architecture Invariant: in project mode, macros and labels are harvested from
> the whole source tree before the entry file is converted, so there are no
> undefined-macro or forward-reference surprises regardless of source order.

`warnings.rs` — `Warning`, `Category`, `Severity`: the `warnings.json` shape.

`skills.rs` — accessor for the repair guides embedded at build time.

`build.rs` — compiles the vendored grammar and embeds `skills/*.md`.

> Architecture Invariant: the repair skills are embedded at build time, so the
> single `byetex` binary ships its whole catalogue and works fully offline.

### `crates/byetex-cli` — the `byetex` binary

`main.rs` — `clap`-based dispatch for the subcommands: `convert` (single file
or `--project`), `agent-brief`, `doctor`, `corpus`, `skills`, and `serve` (the
MCP server, behind the `mcp` feature).

> Architecture Invariant: this layer owns all filesystem I/O and all process
> spawning (`typst`, `tectonic`). The core stays pure.

> Architecture Invariant: a successful `convert` exits 0 regardless of warning
> count, and always writes the `.typ` and `.warnings.json` (even when empty).
> Callers read the sidecar, never the exit code. (`doctor` is the exception: it
> validates input and reports non-zero verdicts.)

### `crates/byetex-mcp` — the MCP server

`lib.rs` — `ByeTexServer` over stdio JSON-RPC (the `rmcp` crate). Each tool —
`convert`, `convert_file`, `convert_fragment`, `convert_project`, `list_skills`,
`read_skill` — is a thin async wrapper over the core.

## Cross-Cutting Concerns

### Diagnostics / warnings

Warnings are a first-class output, not a log. `Category` is a tagged union
(`unsupported_command`, `unsupported_environment`, `ambiguous_math`,
`parse_error`, `tikz`, `custom_macro`, `unknown_package`, `drop_only`,
`needs_manual_review`); each warning carries a source range, a snippet, and a
`suggested_skill` pointing at a repair guide. The JSON shape is documented in
[`warnings.schema.json`](warnings.schema.json) and locked by a regression test.

### Unsupported-construct handling

Four strategies, applied per construct: **emit + warn** (translate
approximately, flag it), **silent blessed-drop** (constructs that are no-ops in
Typst, e.g. spacing/layout hints), **recursive fallback** (`\input`/`\include`
expanded inline with cycle-breaking), and **parse_error** (the grammar
couldn't parse the region — emit what's available and warn).

### Skills

Repair guides authored as `skills/*.md`, embedded at build time, and surfaced
through `byetex skills` and the MCP `list_skills`/`read_skill` tools. A
warning's `suggested_skill` links the problem to its guide.

### Project mode vs single-file mode

Single-file mode is one `convert` call: includes and figures are dropped with
`needs_manual_review`. Project mode (`plan_project` → harvest → convert →
`materialize_project`) is the only mode that resolves `\input` trees, copies
assets, and generates `typst.toml`.

### Codegen

`build.rs` produces two artifacts: the compiled tree-sitter grammar linked by
`parser.rs`, and the generated skill catalogue `include!`-ed by `skills.rs`.

### Testing

Layered: golden snapshot tests (`insta`) pin exact Typst output for fixtures;
a compile-check layer shells out to `typst` to confirm the output compiles; a
schema test locks the `warnings.json` contract. Beyond the unit suite, a
regression corpus of real arXiv papers (`corpus/manifest.json` is committed; the
payloads are gitignored) is swept by `scripts/corpus_sweep.sh`, and
`scripts/visual_test.py` renders side-by-side PDF composites for visual grading.
See [`test-plan.md`](test-plan.md). Run `cargo test --workspace` before claiming
a fix is complete.
