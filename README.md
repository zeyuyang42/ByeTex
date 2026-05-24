# ByeTex

A fast, single-binary LaTeX → Typst converter built for AI agents (and humans).

ByeTex deterministically translates an academic-paper subset of LaTeX into
[Typst](https://typst.app) and, for everything outside that subset, emits a
structured `warnings.json` sidecar plus a bundled catalogue of skill files
that explain how to finish the conversion by hand or with an LLM.

## What it converts cleanly

- Document classes: `article`, `report`. `\documentclass{...}` and `\usepackage{...}` are noted in warnings.
- Sectioning: `\part` / `\chapter` / `\section` / `\subsection` / `\subsubsection` / `\paragraph` / `\subparagraph`, including starred forms.
- Inline formatting: `\emph`, `\textbf`, `\textit`, `\texttt`, `\underline`, `\textsc`.
- Lists: `itemize`, `enumerate`, `description`.
- Math: `$...$`, `\[...\]`, `$$...$$`, `equation`/`equation*`, `align`/`align*`, `gather`, `multline`, `cases`, `pmatrix`/`bmatrix`/`vmatrix`/`matrix`.
- Math symbols: full Greek lower/upper, `\frac`, `\sqrt`, `\binom`, `\sum`, `\int`, `\prod`, common operators (`\cdot`, `\leq`, `\to`, `\infty`, ...), and standard set/logic notation.
- Tables: `tabular` with `l`/`c`/`r` column specs.
- Figures: `figure` env + `\includegraphics[width=...]{path}` + `\caption{...}` + `\label{...}`.
- References: `\label`, `\ref`, `\eqref`, `\pageref`.
- Citations + bibliography: `\cite`, `\bibliography`, `\bibliographystyle`.
- Misc: `%` comments (LaTeX-faithfully consumed), `\\` line breaks, `\noindent` / `\indent`.

Anything else produces a structured warning categorised as
`unsupported_command`, `unsupported_environment`, `tikz`, `custom_macro`,
`parse_error`, `ambiguous_math`, or `needs_manual_review`.

## Install

Pre-built binaries are published with each release for:
`x86_64-linux-musl`, `aarch64-linux-musl`, `x86_64-apple-darwin`,
`aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.

```bash
# Download the latest tarball for your platform from GitHub Releases.
# Each archive includes the `byetex` binary plus the `skills/` directory.
tar -xzf byetex-vX.Y.Z-<target>.tar.gz
./byetex-vX.Y.Z-<target>/byetex --version
```

Or via cargo (requires Rust 1.85+):

```bash
cargo install --git https://github.com/zeyuyang42/ByeTex byetex-cli --features mcp
```

## CLI

```bash
# Convert a LaTeX project FOLDER (recommended for real papers).
# Auto-detects the entry .tex (the one with \documentclass), pre-scans
# every .tex/.sty/.cls in the tree for \newcommand/\def, then converts.
byetex convert ./paper-source
# Writes next to the dir:
#   paper-source.typ
#   paper-source.warnings.json
#   paper-source.agent_brief.md   ← portable Markdown brief for LLM patching

# Convert a single LaTeX document.
byetex convert paper.tex
# Writes paper.typ, paper.warnings.json, and paper.agent_brief.md.

# Skip the brief for batch / CI runs.
byetex convert paper.tex --no-brief

# Inspect the warnings.
cat paper.warnings.json | jq '.[].category.kind' | sort | uniq -c

# Browse the bundled skills.
byetex skills list
byetex skills read byetex-using-warnings-json

# Run as an MCP server over stdio.
byetex serve

# Track regression coverage against a markdown-bundled corpus.
byetex corpus harvest --source context/latex-context.md --out tests/corpus/
byetex corpus run --dir tests/corpus/
```

### Project mode

For real-world LaTeX projects that include figures, bibliography files, and
`\input` sub-files, use `--project` to produce a self-contained Typst project
directory that compiles end-to-end. The input can be either the entry `.tex`
file or the project folder; for arXiv tarballs, point at the unpacked folder
and ByeTex picks the entry plus harvests all sibling `.sty`/`.cls` macros
before converting:

```bash
# Recommended: pass the unpacked arXiv folder.
byetex convert ./paper-source --project
# Writes paper-source.typst-project/ containing:
#   main.typ          — the converted Typst body
#   fig/foo.pdf       — asset files copied from the source project
#   refs.bib          — bibliography copied as-is (Typst reads it natively)
#   typst.toml        — optional manifest for known document classes
#   warnings.json     — structured warnings sidecar
#   agent_brief.md    — portable Markdown brief (paste into an LLM to patch
#                       residual compile errors). Skip with --no-brief.

# Or, if you already know the entry file, pass it directly.
byetex convert paper.tex --project

# Specify a custom output directory.
byetex convert ./paper-source --project --project-out /tmp/my-project

# Skip typst.toml generation.
byetex convert paper.tex --project --no-toml

# Overwrite a non-empty output directory.
byetex convert paper.tex --project --project-out /tmp/my-project --force

# Compile the result.
typst compile paper.typst-project/main.typ
```

## Output contract

Every `byetex convert` writes two files next to the input:

- `<stem>.typ` — the converted Typst document.
- `<stem>.warnings.json` — an array of warnings, even if empty, so downstream
  tools can rely on the file existing.

The JSON schema is fully documented at [`docs/warnings.schema.json`](docs/warnings.schema.json)
and the public shape is locked by a regression test
(`crates/byetex-core/tests/warnings_schema.rs`).

A representative warning:

```json
{
  "range": {
    "start_line": 42, "start_col": 1,
    "end_line": 47,  "end_col": 18,
    "byte_start": 1023, "byte_end": 1184
  },
  "category": { "kind": "tikz" },
  "severity": "warning",
  "message": "...",
  "snippet": "\\begin{tikzpicture}...\\end{tikzpicture}",
  "suggested_skill": "byetex-tikz-to-typst"
}
```

`severity` is `info | warning | error`. The exit code of `byetex convert` is
**always 0** when conversion succeeds — even with warnings — so callers should
inspect the sidecar rather than the exit code.

## For AI agents

See [`docs/for-agents.md`](docs/for-agents.md). The short version:

1. `byetex convert input.tex` is non-destructive and idempotent.
2. Read `input.warnings.json`. Empty means a clean conversion.
3. Each warning's `suggested_skill` points to a markdown file in `skills/`
   that documents how to resolve that warning category. Reach the skills via
   `byetex skills read <name>`, by opening `skills/<name>.md` on disk, or
   over MCP with the `read_skill` tool.
4. For interactive use, `byetex serve` exposes the converter and skills as
   MCP tools (`convert`, `convert_file`, `convert_fragment`, `list_skills`,
   `read_skill`).

## Project layout

```
ByeTex/
├── crates/
│   ├── byetex-core/    # parser, IR, emitter, warnings, skills
│   ├── byetex-cli/     # `byetex` binary
│   └── byetex-mcp/     # rmcp-backed MCP server
├── context/             # LaTeX & Typst reference docs (corpus source)
├── skills/              # bundled markdown skills, embedded at build time
├── tests/fixtures/      # per-milestone golden test inputs
└── docs/
    ├── for-agents.md
    └── warnings.schema.json
```

## Status

Current corpus pass-rate (clean + warnings) against the harvested
`context/latex-context.md` blocks: **87%**, with 13% in `parse_error` (the
tree-sitter-latex grammar is best-effort and gives up on some exotic TeX).

The supported subset is meant to grow incrementally; each release will bump
the corpus threshold.

## License

Dual-licensed under MIT or Apache 2.0. Vendored sources keep their original
licenses (notably `crates/byetex-core/vendor/tree-sitter-latex/LICENSE`).
