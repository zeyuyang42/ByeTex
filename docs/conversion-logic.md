# ByeTex Conversion Logic: Architecture Overview

This document is the box-and-arrow view of how `byetex` turns a LaTeX
project into a Typst project. It's aimed at new contributors who need a
mental model before opening `emit.rs` (5400 lines and counting).
Per-function detail lives in the source.

## 1. The crates

```
byetex-cli  ──── thin layer ────► byetex-core  ◄──── byetex-mcp
   binary `byetex`                 the converter          rmcp server
```

- **`byetex-core`** is the library: parser, emitter, project planner,
  warnings, skills. Pure functions; no filesystem access except via
  explicitly-passed `base_dir`.
- **`byetex-cli`** is the binary. It owns argument parsing, decides
  which mode to run in (flat / project; file / folder), and handles
  all filesystem I/O — including the project materializer and the
  agent-brief writer.
- **`byetex-mcp`** is an alternative front-end exposing the same
  conversion primitives as MCP tools (`convert`, `convert_file`,
  `convert_project`, `list_skills`, `read_skill`).

Everything below describes byetex-core's logic, then how the two
front-ends call into it.

## 2. Input shapes and output shapes

A user can hand ByeTex four combinations:

| Input | Flag | Output |
|---|---|---|
| `.tex` file | (none) | `<stem>.typ` + `<stem>.warnings.json` + `<stem>.agent_brief.md` |
| `.tex` file | `--project` | `<stem>.typst-project/` (with `main.typ`, assets, manifest, sidecars) |
| folder | (none) | `<dirname>.typ` + `<dirname>.warnings.json` + `<dirname>.agent_brief.md` |
| folder | `--project` | `<dirname>.typst-project/` |

The folder forms detect the entry `.tex` (the one with `\documentclass`)
automatically and pre-scan every `.tex` / `.sty` / `.cls` in the tree
for macros before conversion. The flat forms drop figures and bib
files; project mode copies them into the output dir.

## 3. Parse → emit pipeline

```
                 ┌──────────────┐
   source &str ──► tree-sitter  │ parser::parse() returns a Tree
                 │  -latex      │ (vendored grammar, no separate parse pass)
                 └──────┬───────┘
                        │ AST root node
                 ┌──────▼───────┐
                 │   Emitter    │ state: out: String, in_math: bool,
                 │              │        macros, asset_refs, warnings,
                 │              │        detected_class, metadata,
                 │              │        skip_until, macro_depth, ...
                 │              │
                 │ emit_root()  │ walks the tree once, depth-first
                 │              │ via `emit_node()` dispatch
                 └──────┬───────┘
                        │
                 ┌──────▼───────┐
                 │   finish()   │ prepends template preamble (#import +
                 │              │ #show: X.with(...)) or hand-rolled
                 │              │ title block; prepends conditional
                 │              │ #set heading / equation numbering;
                 │              │ runs post_process_typography (dashes,
                 │              │ smart quotes); returns the four-tuple.
                 └──────────────┘
```

**`Emitter::emit_node`** in `crates/byetex-core/src/emit.rs` is the
dispatch table — a large `match` on tree-sitter node kinds. Important
branches:

- `text` / `word` — copied through verbatim with light typography fixes.
- `generic_command` and bare `command_name` — looks up the LaTeX
  command name, dispatches to one of dozens of `emit_*` helpers
  (`\title`, `\section`, `\frac`, `\includegraphics`, …) or falls
  through to `expand_user_macro` if the name is in `self.macros`,
  else `warn_unsupported_command`.
- `inline_formula`, `displayed_equation`, `math_environment` — flips
  `in_math = true`, calls `emit_math_children` which routes math-mode
  commands through `lookup_math_symbol`, `emit_math_wrap`,
  `emit_math_frac`, etc.
- `package_include` / `class_include` — pulls macros into the table
  (see §5).
- `latex_input` (\input/\include) — recursively expands via a child
  Emitter.

**Math vs text mode** is tracked solely by `Emitter::in_math`. Both
modes share many helpers; the boolean determines whether to emit
`\alpha` as `alpha` (math) or warn (text), whether `_` is subscript or
literal, etc.

**`skip_until`** is the field that lets emit-time helpers consume
source bytes past `node.end_byte()` — used for brace-less argument
forms where the AST sibling is technically separate but conceptually
part of the call (`\mat X`, `\hat\alpha`).

## 4. Project mode + asset handling

Project mode is a two-layer design in `crates/byetex-core/src/project.rs`
+ `crates/byetex-cli/src/project.rs`:

```
            file input                folder input
                │                          │
                ▼                          ▼
       ┌────────────────┐         ┌──────────────────┐
       │  plan_project  │         │ plan_project_    │
       │                │         │   from_dir       │
       │ - read .tex    │         │ ┌──────────────┐ │
       │ - convert()    │         │ │detect_entry_ │ │
       │ - build plan   │         │ │  file()      │ │ ◄── recursive walk,
       │                │         │ └──────┬───────┘ │     finds the .tex
       └────────┬───────┘         │        ▼         │     with \documentclass
                │                 │ ┌──────────────┐ │
                │                 │ │harvest_      │ │ ◄── pre-scan every
                │                 │ │ project_     │ │     .tex/.sty/.cls
                │                 │ │  macros()    │ │     for \newcommand
                │                 │ └──────┬───────┘ │
                │                 │        ▼         │
                │                 │ convert_with_    │
                │                 │   macros() ──────┘
                │                          │
                ▼                          ▼
                       ProjectPlan {
                         main_typst: String,
                         assets:     Vec<AssetCopy>,
                         warnings:   Vec<Warning>,
                         manifest:   Option<String>,
                         entry_tex:  PathBuf,
                       }
                                   │
                                   ▼
                       materialize_project()
                       (in byetex-cli or byetex-mcp)
                       - writes main.typ
                       - copies assets through the path-
                         traversal guard
                       - writes typst.toml + warnings.json
                       - writes agent_brief.md (unless --no-brief)
```

**`AssetRef` plumbing.** During emit, every `\includegraphics{...}`
and `\bibliography{...}` whose target file exists on disk records an
`AssetRef { kind, typst_path, source_path }` on the Emitter. Sub-
emitters (from `\input`-expansion and from brace-less math-wrap of a
user macro body) merge their `asset_refs` back into the parent. The
project planner turns each `AssetRef` into an `AssetCopy` whose
`rel_dest` mirrors the path string the emitter wrote into the Typst
source, so `image("fig/foo.pdf")` keeps working after relocation.

**Path-traversal guard.** `materialize_project` canonicalises both
`base_dir` and each asset's source path, then refuses to copy any
asset whose canonical source escapes `base_dir`. Hardened in both
the CLI materializer and the MCP `materialize_project_mcp`: empty
parent dirs are normalised to `"."`, canonicalisation failures error
out rather than silently dropping every asset, and `--force` cleans
the existing output dir before writing so stale files don't survive.

## 5. Macros, packages, class templates

These three subsystems all populate `Emitter::macros` and
`Emitter::detected_class`:

```
\newcommand{\foo}[1]{...}  ──►  Emitter.macros[\foo] = MacroDef
\usepackage{mypkg}         ──►  read mypkg.sty next to source,
                                  harvest \newcommand/\def from it
                                  via expand_local_package
\usepackage{physics}       ──►  bundled MacroSeed table in
                                  package_macros.rs seeds \dv, \pdv,
                                  \bra, \ket, ...
\documentclass{IEEEtran}   ──►  DocClass::IeeeTran detected, drives
                                  build_template_preamble() to emit
                                  #import "@preview/charged-ieee:..." +
                                  #show: ieee.with(title, authors, ...)
```

**Two-pass model (folder mode only).** `harvest_project_macros`
walks every `.tex`/`.sty`/`.cls` in the project tree, parses each
file standalone, harvests every `\newcommand` / `\def` into one
merged table, and seeds the Emitter via `with_includes_and_macros`
before the main walk. The walk-time `\newcommand` parse still runs
on top — locally-defined macros at the call site win against the
pre-scan because the walk uses `or_insert`. This guarantees that
macros defined in a sibling file the entry never `\input`s are
still available at every call site.

**Macro expansion** (`expand_user_macro` in `emit.rs`):
1. Look up the macro in `self.macros`. If absent, drop with no-op.
2. Collect curly-group args from the AST.
3. If fewer args than `macro_def.params`, fall back to brace-less
   consumption: `consume_braceless_arg(src, i)` reads the next
   `\command` / `{group}` / single codepoint from the raw source
   bytes. This is what makes `$\mat X$` work (where `\mat` is a
   1-arg `\newcommand`).
4. Substitute `#1..#N` placeholders in the body via a tokenising
   walker (so `#10` isn't matched by `#1`).
5. Re-parse the substituted body and emit through a sub-Emitter
   that inherits math context, base_dir, and a depth counter.
6. Merge the sub-emitter's `out` / `warnings` / `asset_refs` /
   defined-macros back into the parent.
7. A `MAX_MACRO_DEPTH` (64) cap prevents self-referential macros
   from overflowing the stack.

**Bundled package library** (`crates/byetex-core/src/package_macros.rs`)
holds `MacroSeed { params, body }` tables for `physics`, `bm`,
`stmaryrd`, `mathtools`. When `\usepackage{<pkg>}` is encountered,
`expand_local_package` runs first (local `.sty` wins), then the
bundled table is merged via `or_insert_with` (loses to anything
already in the table). Note: packages on the
`is_known_noop_package` list at `emit.rs` short-circuit *before*
the bundled-seed lookup — a known wart, with a documented fix in
the deferred plan. (Expanding the bundled library to siunitx /
hyperref / cleveref / mhchem / xcolor / booktabs is the largest
TODO outstanding.)

**Class templates** (`crates/byetex-core/src/class_map.rs`).
`DocClass::from_class(name)` maps `IEEEtran`, `acmart`, `neurips`,
`icml`, `iclr`, `revtex`, `elsarticle`, `arxiv`, `llncs`, `svmono`,
etc. to a `DocClass` variant. `\usepackage{...}` can refine it
(e.g. `\usepackage{neurips_2024}` upgrades `Unknown` to `Neurips`).
At `finish()` time, `build_template_preamble` calls
`detected_class.import_line()` + `detected_class.show_call(meta)`
to emit the `#import "@preview/charged-ieee:0.1.4": *` and
`#show: ieee.with(title: ..., authors: (...))` pair. The metadata
(`title`, `authors`, `abstract`, `keywords`, affiliations, ORCIDs,
date) is captured incrementally during the walk into a
`DocumentMetadata` struct in `document.rs`. Per-class author
parsers handle the wildly different `\author` conventions
(IEEE's `\IEEEauthorblockN/A`, NeurIPS's `\And`-separated lists,
the generic `\author{Alice \and Bob}`).

Show-call slot escaping: each template's show-call routes user-
supplied strings through `string_escape` (for `"..."` slots) or
`content_escape` (for `[...]` slots) so `\"`, `\\`, `]` in author
metadata don't break the generated Typst.

## 6. Warnings, brief, sidecars

Everything ByeTex couldn't convert cleanly becomes a `Warning`:

```rust
struct Warning {
    range: Range,             // line/col + byte offsets in the source
    category: Category,       // tagged enum, see below
    severity: Severity,       // info | warning | error
    message: String,
    snippet: String,          // the offending source bytes
    suggested_skill: Option<String>,  // name of a skills/*.md file
}
```

**Categories** (`crates/byetex-core/src/warnings.rs`):

| Kind | Triggered by |
|---|---|
| `unsupported_command` | A `\command` ByeTex doesn't model and isn't a user macro. |
| `unsupported_environment` | `\begin{X}...\end{X}` for an unknown X. |
| `tikz` | `tikzpicture` env — emits a placeholder rect + this warning. |
| `custom_macro` | A `\newcommand` call we couldn't satisfy (param-count mismatch or genuinely missing arg after the brace-less fallback). |
| `parse_error` | tree-sitter-latex failed on the source. |
| `ambiguous_math` | A math construct we recognise structurally but can't render confidently. |
| `needs_manual_review` | Catch-all for known-blocking patterns (missing image, unbalanced delimiter, etc.). Usually carries `suggested_skill`. |

**Sidecars** written by `byetex convert` (and by every flow that
uses `write_agent_brief`):

- `<stem>.typ` / `<out>/main.typ` — the converted body.
- `<stem>.warnings.json` / `<out>/warnings.json` — the warnings array.
  Pretty-printed JSON; even an empty doc gets `[]`.
- `<stem>.agent_brief.md` / `<out>/agent_brief.md` — a Markdown
  bundle for LLM remediation. Contains the source `.tex`, the
  generated `.typ`, optional `typst compile` log, all warnings
  with their categories histogrammed, and a `<stem>_manual.typ`
  pointer the LLM should write its patched copy to. Paths in the
  brief are rendered relative to the brief's own directory so the
  doc stays portable. On by default; `--no-brief` suppresses.
- `<out>/typst.toml` (project mode only) — derived in
  `derive_manifest` by peeking the first few lines of the
  generated `.typ` for an `#import "@preview/X:V"` line and
  building a minimal package manifest.

The `byetex agent-brief` subcommand is now a thin wrapper that
runs the same convert flow with the brief always on and the
`typst compile` step actually invoked (vs. skipped in plain
`convert`).

## 7. Where to look for what

A cheat-sheet of file → responsibility:

```
crates/byetex-core/src/
├── lib.rs           ConvertOptions, ConvertOutput, convert() entry
├── parser.rs        tree-sitter-latex parser wrapper
├── emit.rs          THE big file: Emitter + emit_node dispatch +
│                    50+ emit_* helpers + helpers for path probing,
│                    macro substitution, brace-less arg consumption,
│                    typography post-processing
├── document.rs      DocumentMetadata, Author, Content, etc.
├── class_map.rs     DocClass enum, per-class show-call builders,
│                    author-block parsers
├── project.rs       ProjectPlan, plan_project[_from_dir],
│                    detect_entry_file, harvest_project_macros
├── package_macros.rs  bundled MacroSeed tables (physics, bm, ...)
├── warnings.rs      Warning / Category / Severity types
└── skills.rs        embedded skills/*.md catalog

crates/byetex-cli/src/
├── main.rs          clap subcommands, run_convert{_project,_dir_flat,
│                    _agent_brief}, write_agent_brief + BriefInputs
└── project.rs       materialize_project (writes the plan to disk,
                     runs the path-traversal guard, cleans on --force)

crates/byetex-mcp/src/
└── lib.rs           rmcp server + materialize_project_mcp (mirror
                     of the CLI materializer to avoid a circular dep)
```

For "where does X go wrong" debugging, the most productive entry
points are:

1. `emit_node` in `emit.rs` — every conversion failure originates here.
2. `expand_user_macro` — for missing-macro / wrong-arg warnings.
3. `harvest_project_macros` — for "my `\newcommand` isn't being seen".
4. `build_template_preamble` + `DocClass::show_call` in
   `class_map.rs` — for title-block / author-list problems.
5. `materialize_project` in `byetex-cli/src/project.rs` — for asset
   copying / path-traversal complaints.
