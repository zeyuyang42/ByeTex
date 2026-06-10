# Using ByeTex from an AI agent

This document is the canonical entry point for AI coding agents (Claude Code,
Cursor, Codex, etc.) that want to convert a LaTeX document to Typst as part
of a larger workflow.

## Four invariants

1. **`byetex convert input.tex` exits 0 on success**, even when warnings are
   emitted. Inspect the sidecar JSON, not the exit code.
2. **Warnings live in `<stem>.warnings.json`** next to the `.typ`. The file
   is always written, even if empty (`[]`).
3. **Skills are reachable in three ways**:
   - `byetex skills list` and `byetex skills read <name>` from the CLI.
   - `skills/<name>.md` files in the release archive (or this repo).
   - The `list_skills` and `read_skill` MCP tools when running `byetex serve`.
4. **The output `.typ` is always written.** Even if some constructs are
   unconvertible, ByeTex emits something — possibly with `#text(red)[\foo]`
   placeholders — and points you at the warning + skill needed to repair it.

## Quickstart

```bash
# Download and extract a release tarball (single static binary):
curl -sSL -o byetex.tar.gz https://github.com/zeyuyang42/ByeTex/releases/latest/download/byetex-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz
tar -xzf byetex.tar.gz
cd byetex-vX.Y.Z-x86_64-unknown-linux-musl

# Convert:
./byetex convert paper.tex

# Inspect:
cat paper.warnings.json | jq '.[].category.kind' | sort | uniq -c
typst compile paper.typ
```

## Workflow

When the goal is **"make it compile"**, the [diagnose-first repair loop](#repair-loop)
below is the headline path — `byetex diagnose` compiles the output and maps each
error back to its LaTeX fragment + skill. The `convert` + `warnings.json` flow shown
here is the lower-level path for inspecting conversion gaps that compile but render
approximately.

```
   ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
   │  paper.tex      │──────▶│ byetex convert │──────▶│  paper.typ      │
   └─────────────────┘       └─────────────────┘       │  paper.warnings.│
                                      │                │      json       │
                                      ▼                └─────────────────┘
                             ┌─────────────────┐                │
                             │ byetex skills  │                │
                             │ read <suggested>│◀───────────────┘
                             └─────────────────┘
                                      │
                                      ▼
                             Edit paper.typ at the warned ranges
                                      │
                                      ▼
                             typst compile paper.typ
```

1. Run `byetex convert input.tex`.
2. If `input.warnings.json` is `[]`, you're done — `typst compile` and move
   on.
3. Otherwise:
   1. Group warnings by `category.kind`.
   2. For each kind, read the file named by `suggested_skill` (when set).
      That skill explains the resolution pattern.
   3. Apply edits to the `.typ` at the byte ranges given.
   4. Re-run `typst compile input.typ`.

## MCP server mode

For interactive use, the converter speaks MCP over stdio:

```bash
./byetex serve
```

The seven tools exposed:

| Tool                | Purpose                                                        |
|---------------------|----------------------------------------------------------------|
| `convert`           | Convert a LaTeX string in-memory, get `{typst, warnings}`.     |
| `convert_file`      | Convert a `.tex` path, write `.typ` + sidecar, return paths.   |
| `convert_fragment`  | Convert a snippet with a `context_hint` (inline / block / math). |
| `convert_project`   | Convert a multi-file project to a self-contained Typst dir.    |
| `diagnose`          | Compile the output and map each typst error to its LaTeX fragment + skill. |
| `list_skills`       | List bundled skills (`name`, `description`).                   |
| `read_skill`        | Read a skill's full markdown body.                             |

## Reading `warnings.json`

The complete JSON schema is at [`warnings.schema.json`](warnings.schema.json).
A minimal recipe:

```bash
# Total warnings
jq 'length' paper.warnings.json

# Group by category
jq 'group_by(.category.kind) | map({kind: .[0].category.kind, count: length})' paper.warnings.json

# Pretty-print warnings with their skill suggestions
jq '.[] | {line: .range.start_line, kind: .category.kind, skill: .suggested_skill, snippet}' paper.warnings.json
```

Categories you will see:

- `unsupported_command` — a backslash command outside the v1 subset (e.g. `\marginpar`, `\title`).
- `unsupported_environment` — a LaTeX environment outside the v1 subset.
- `custom_macro` — `\newcommand` / `\def`. Rare path; ByeTex passes them through.
- `tikz` — TikZ picture; CeTZ migration recommended.
- `parse_error` — tree-sitter could not parse that region.
- `ambiguous_math` — math command without a Typst equivalent. The `.typ` will
  contain a `#text(red)[\foo]` placeholder at the position. Read
  `byetex skills read byetex-math`.
- `unknown_package` — `\usepackage{...}` with no known mapping.
- `drop_only` — benign drop, already handled.
- `needs_manual_review` — converted approximately; verify against the original PDF.

## Recovering from `parse_error`

These usually come from:

- Mismatched `{` / `}` in the original `.tex`.
- `\verb` with unusual delimiters.
- Custom `\def` that produces unbalanced output.

Read `byetex skills read byetex-parse-error` for the full procedure.

## Repair loop

When a converted `.typ` does not compile, use `byetex diagnose` to drive a
targeted fix cycle. For the full procedure read the bundled skill:

```bash
byetex skills read byetex-repair-loop
```

Outline:

```
byetex diagnose paper.tex
  → paper.typ + paper.diagnostics.json  (per error: src_fragment, typ_region, skill_name)
  → for each error: read skill, edit paper.typ
  → typst compile paper.typ  ──(errors?)──┐
        ▲                                  │ loop until clean
        └──────────────────────────────────┘
```

Key rule: **do not re-run `byetex diagnose` between edits** — it overwrites
`paper.typ` from source, discarding your fixes. Use `typst compile paper.typ`
to iterate; re-run `diagnose` only to start over from the LaTeX.

## When ByeTex isn't enough

If too many warnings have `needs_manual_review` and you can't make progress,
the best escape hatch is to render the original LaTeX fragment to PDF/SVG
using `pdflatex` or `tectonic`, then `#image("frag.pdf")` from Typst. This is
documented in `byetex-unsupported-environment.md`.
