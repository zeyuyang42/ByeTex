# ByeTex

A fast, single-binary LaTeX → Typst converter built for AI agents (and humans).

ByeTex deterministically translates an academic-paper subset of LaTeX into
[Typst](https://typst.app) and, for everything outside that subset, emits a
structured `warnings.json` sidecar plus a bundled catalogue of skill files
that explain how to finish the conversion by hand or with an LLM.

## What it converts cleanly

**Document classes** — `article`/`report`/`book` (arkheion arXiv template),
`IEEEtran`/`IEEEconf` (charged-ieee), `acmart` (clean-acmart),
`revtex4`/`revtex4-1`/`revtex4-2` (revtyp), `elsarticle` (elsearticle),
`llncs`/`svmult` (Springer LNCS), NeurIPS/ICML/ICLR (detected via style
packages → lucky-icml). Format options (`sigconf`, `journal`, `conference`,
…) are forwarded to the matching Typst Universe template.

**Sectioning** — `\part` / `\chapter` / `\section` / `\subsection` /
`\subsubsection` / `\paragraph` / `\subparagraph`, including starred forms.

**Inline formatting** — emphasis (`\emph`, `\textbf`, `\textit`, `\textsc`,
`\underline`), monospace (`\texttt`), sub/superscripts
(`\textsuperscript`/`\textsubscript`), color (`\textcolor`), links
(`\href`/`\url`), boxes (`\mbox`/`\fbox`), text symbols (`\S`, `\P`,
`\copyright`, `\ldots`, `\today`, …).

**Lists** — `itemize`, `enumerate`, `description`.

**Math** — any AMSMath display environment (`equation`/`align`/`gather`/
`multline`/`alignat`/`flalign`/`split`/`cases` and starred variants),
`subequations` with label staging, matrix family (`matrix`/`pmatrix`/
`bmatrix`/`vmatrix`/`Vmatrix`/`Bmatrix`/`smallmatrix`), inline `$…$` /
`\(...\)` / `$$…$$` / `\[…\]`. ~450 symbols/operators: full Greek,
blackboard/calligraphic/fraktur fonts (`\mathbb`, `\mathcal`, `\mathfrak`,
`\mathscr`), arrows, accents (`\bar`, `\hat`, `\vec`, `\widehat`, …),
brackets, layout primitives (`\stackrel`, `\xrightarrow`, `\substack`,
`\smash`, `\phantom`), and full trig/log function names.

**Tables** — `tabular`/`tabular*`/`array`/`tblr` (tabularray)/`tabularx`/
`tabulary` with `l`/`c`/`r`/`p{w}`/`m{w}`/`b{w}`/`X` columns,
`@{}`/`!{}`/`>{}`/`<{}` inter-column decorators, booktabs rules
(`\toprule`/`\midrule`/`\bottomrule`/`\cmidrule`), `\multicolumn`/
`\multirow`/`\makecell`.

**Figures & floats** — `figure`/`figure*`, `table`/`table*`, `algorithm`/
`algorithm*`/`algorithm2e`, `wrapfigure`/`wraptable` (degrade to standard
float). `\includegraphics[width=…]{path}` + `\caption` + `\label`.

**Theorems** — `theorem`/`lemma`/`corollary`/`proposition`/`definition`/
`example`/`remark`/`proof` plus user-defined kinds harvested from
`\newtheorem`, `\newtcolorbox`, `\newmdenv`.

**Code listings** — `lstlisting`/`verbatim`/`minted` → `#raw(…, block: true)`;
inline `\verb|…|` → `#raw(…)`.

**References** — `\label`, `\ref`, `\eqref`, `\pageref`, `\cref`/`\Cref`
(cleveref).

**Citations & bibliography** — natbib/biblatex-style `\cite`/`\citet`/`\citep`
key lists, `\bibliography{…}` with `.bib` preprocessing for Hayagriva,
`\bibliographystyle{…}` mapped to Typst styles, `.bbl` fallback when only the
pre-rendered file is bundled. `\bibitem` harvested for key registration.

**Custom macros** — `\newcommand`/`\renewcommand`/`\def`/`\newcommandx`
pre-scanned from every `.tex`/`.sty`/`.cls` in the project tree before
conversion.

**Misc** — `%` comments, `\\` line breaks, `\noindent`/`\indent`, `\footnote`,
`\thepage`/`\thesection`/`\thesubsection`/… → `#context counter(…).display()`,
`\hologo{…}` logo expansion, counter display commands.

Anything else produces a structured warning categorised as
`unsupported_command`, `unsupported_environment`, `drop_only`,
`unknown_package`, `tikz`, `custom_macro`, `parse_error`, `ambiguous_math`,
or `needs_manual_review`.

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
# --features mcp is needed only if you want `byetex serve` (MCP server).
cargo install --git https://github.com/zeyuyang42/ByeTex byetex-cli --features mcp
```

## CLI

```bash
# Convert a LaTeX project FOLDER (recommended for real papers).
# Auto-detects the entry .tex, pre-scans every .tex/.sty/.cls for
# \newcommand/\def, then converts.
byetex convert ./paper-source
# Writes next to the dir:
#   paper-source.typ
#   paper-source.warnings.json
#   paper-source.agent_brief.md   ← portable Markdown brief for LLM patching

# Convert a single LaTeX document.
byetex convert paper.tex

# Write output to a specific path instead of next to the input.
byetex convert paper.tex --output /tmp/out.typ

# Skip the brief for batch / CI runs.
byetex convert paper.tex --no-brief

# Inspect the warnings.
cat paper.warnings.json | jq '.[].category.kind' | sort | uniq -c

# Browse the bundled skills.
byetex skills list
byetex skills read byetex-using-warnings-json

# Run as an MCP server over stdio (requires --features mcp at build time).
byetex serve

# Run the regression corpus over the synthetic test corpus.
byetex corpus run --dir tests/corpus/

# For the arXiv regression corpus (pinned papers from corpus/manifest.json),
# see scripts/README.md and:
#   uv run --with requests python scripts/corpus_harvest.py --pinned
#   ./scripts/corpus_sweep.sh
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

### Agent brief

`byetex agent-brief` is like `convert` but also runs `typst compile` and
captures the compiler log into the brief, giving an LLM a ready-to-patch
report in one step:

```bash
byetex agent-brief paper.tex
byetex agent-brief ./paper-source --project --project-out /tmp/proj

# Skip the typst compile step (produce the brief from warnings only).
byetex agent-brief paper.tex --no-compile
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
  "category": { "kind": "unsupported_command", "name": "\\chemfig" },
  "severity": "warning",
  "message": "...",
  "snippet": "\\chemfig{...}",
  "suggested_skill": "byetex-using-warnings-json"
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
   six MCP tools: `convert`, `convert_file`, `convert_fragment`,
   `convert_project`, `list_skills`, `read_skill`.

## Project layout

```
ByeTex/
├── crates/
│   ├── byetex-core/    # parser, IR, emitter, warnings, skills
│   ├── byetex-cli/     # `byetex` binary
│   └── byetex-mcp/     # rmcp-backed MCP server (feature: mcp)
├── corpus/             # arXiv regression corpus
│   └── manifest.json   # 26 papers (5 pinned); payloads gitignored
├── scripts/            # corpus_harvest.py, visual_test.py, corpus_sweep.sh,
│                       # render_corpus_summary.py — see scripts/README.md
├── skills/             # bundled markdown skills, embedded at build time
├── tests/
│   ├── corpus/         # synthetic doc snippets (outputs gitignored)
│   ├── fixtures/       # per-milestone golden test inputs
│   └── visual/         # rasterized PDF composites (gitignored)
├── vendor/             # vendored tree-sitter-latex (MIT, Patrick Förster 2021)
└── docs/
    ├── for-agents.md
    └── warnings.schema.json
```

## Status

<!-- corpus-summary:start -->
_Last updated: 2026-06-02 (commit 7b6b15f)_

Corpus pass-rate (clean + warnings): **87%** — 431/495 files.

| Bucket | Count |
|---|---:|
| Total | 495 |
| Clean | 200 |
| Warnings (≥1, no parse error) | 231 |
| Parse errors | 64 |

| Warning category | Count |
|---|---:|
| `unsupported_command` | 413 |
| `drop_only` | 87 |
| `unsupported_environment` | 52 |
| `ambiguous_math` | 27 |
| `needs_manual_review` | 7 |
<!-- corpus-summary:end -->

The supported subset is meant to grow incrementally; each release will bump
the corpus threshold.

For arXiv paper compile-fidelity tracking, see the visual regression series under
`docs/visual-regression-*.md`. The most recent snapshot is
[`docs/visual-regression-2026-05-25c.md`](docs/visual-regression-2026-05-25c.md),
which documents Bugs #30–#41 closed and the deferred open tier (#32, #34).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) — your choice.

Vendored and third-party dependency licenses are documented in [NOTICE](NOTICE).
The only vendored source is `crates/byetex-core/vendor/tree-sitter-latex/`
(MIT, Patrick Förster 2021). All 150 Rust cargo dependencies are MIT,
Apache-2.0, Unlicense, or Zlib.
