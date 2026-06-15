# ByeTex

**A LaTeX → Typst converter built for the AI era.**

ByeTex pairs a fast, deterministic core with a native agent loop. The core does the
reproducible heavy lifting — no LLM inside, no network, no guessing — and for the last
mile it can't finish alone, it hands off to an AI agent with surgical, source-mapped
repair instructions. The [Typst](https://typst.app) it emits is good enough to trust:
hand-rolled native math and class-faithful layout. It works best on academic papers
today — where its fidelity is tuned — and the approach generalizes outward.

## Why ByeTex

- **Natively AI-in-the-loop.** Not a wrapper around an LLM — a deterministic tool
  *designed* to be finished by one. When the output doesn't compile, `byetex diagnose`
  maps every Typst error back to the exact LaTeX fragment that caused it and names the
  repair skill that fixes it: the agent gets a worklist, not a stack trace. It ships as
  **7 MCP tools**, **12 bundled repair skills**, and an [`AGENTS.md`](AGENTS.md)
  cold-start — drop it into Claude Code or Cursor with no glue.
- **Best-in-class math.** ByeTex hand-rolls LaTeX → Typst math instead of delegating to
  an external engine — ~450 symbols/operators, coverage gated against the **entire KaTeX
  command set**, emitting **native, editable** Typst math (not images). Hand-rolling wins
  on fidelity: correct accents, no split digits, real Typst you can keep editing.
- **Deterministic & pure.** The core is a pure function — same input, same output, every
  time, with no model inside the binary. The AI only touches clearly-marked edges. That's
  what makes the loop trustworthy: AI leverage without AI unpredictability where it counts.
- **Fidelity, measured.** Beyond "it compiles": a per-class `StyleProfile` (NeurIPS, ICML,
  ICLR, IEEE, ACM, LNCS, …) reproduces title/abstract/heading styling, and a vision agent
  grades the render against an explicit [rubric](docs/fidelity-rubric.md). Compile-rate is
  the gate; visual fidelity is the driver.
- **Never hard-fails.** Anything ByeTex can't translate becomes a visible placeholder plus
  a structured warning — it always produces usable output and exits 0, so a pipeline never
  breaks on a surprise.
- **Real-project aware.** Multi-file `\input`, asset copying, `.bib` preprocessing, and
  macro pre-scan across every `.tex`/`.sty`/`.cls` — it converts real arXiv tarballs, not
  just toy single files.

## How it works

A conversion is a single deterministic forward pass — there is no intermediate IR, and
it never hard-fails:

```
.tex ─┐
      ▼
  parse      tree-sitter-latex → concrete syntax tree                 parser.rs
      ▼
  prepass    harvest the doc class, \newcommand/\def/\newif, title,
             authors, abstract, \label/\ref targets, bib keys         emit.rs::prepass_collect
      ▼
  emit       single forward walk; each node dispatched to a
             specialized emitter — math, tables, figures, sections,
             bibliography, … (15 emit/ submodules)                    emit.rs + emit/*.rs
      ▼
  finish     prepend a self-contained neutral preamble + a per-class
             StyleProfile (title / abstract / heading sizes)          emit/preamble.rs, style_profile.rs
      ▼
  project    --project: copy assets, preprocess .bib, resolve
             \input, write typst.toml                                 project.rs
      ▼
 .typ  +  warnings.json   (+ agent_brief.md, + diagnostics.json)
```

Anything outside the supported subset degrades gracefully: the construct becomes a
visible placeholder (e.g. `#text(red)[\chemfig…]`) plus a structured warning — never a
crash. `byetex convert` exits **0 even with warnings**, so callers inspect the sidecar,
not the exit code.

## Agent-in-the-loop

ByeTex does the deterministic majority of the work and hands the residual off to an
**external AI agent** (Claude Code, Cursor, …) over two feedback loops. No model runs
inside the binary — ByeTex stays a pure tool; the agent drives.

**1. Compile-repair loop — make it compile.**

```bash
byetex diagnose paper.tex      # convert → typst compile → map each error back to its
                               # LaTeX fragment + a repair skill
                               #   → paper.typ + paper.diagnostics.json
```

`diagnostics.json` is a *content-anchored* source map: each `typst` error carries the
originating LaTeX fragment and a `skill_name`. The agent reads the skill
(`byetex skills read <name>`), edits `paper.typ`, and re-runs `typst compile` until
clean. (Don't re-run `diagnose` mid-edit — it re-converts and overwrites your edits.)

**2. Visual-fidelity loop — make it *look* right.**

Rasterize the rendered PDF, then have a vision agent grade it against an explicit rubric
([`docs/fidelity-rubric.md`](docs/fidelity-rubric.md)) — title/abstract styling, citation
forms, float placement, page density — emitting a per-dimension verdict. Findings roll up
into the ranked [`docs/fidelity-backlog.md`](docs/fidelity-backlog.md). This loop catches
typography and layout defects that compile-only and word-recall metrics are blind to.

If you're an agent, **start with [`AGENTS.md`](AGENTS.md)** — the cold-start orientation.

## What it converts

A condensed view; see [`docs/architecture.md`](docs/architecture.md) and
[`docs/conversion-logic.md`](docs/conversion-logic.md) for the exhaustive per-command list.

- **Document classes** — `article`/`report`/`book`, `IEEEtran`/`IEEEconf`, `acmart`,
  `revtex4-*`, `elsarticle`, `llncs`/`svmult` (Springer), and NeurIPS/ICML/ICLR (detected
  via style packages). ByeTex emits a **self-contained neutral preamble** that compiles on
  stock Typst with no `@preview` imports; the detected class drives a per-class
  **`StyleProfile`** (title size/weight/rules, abstract style + in-column placement, heading
  sizes, citation form, bibliography style, body font).
- **Sectioning & inline** — every heading level incl. starred forms; emphasis, `\texttt`,
  sub/superscripts, `\textcolor`, links (`\href`/`\url`), boxes, text symbols.
- **Lists** — `itemize`, `enumerate`, `description`.
- **Math** — every AMSMath display environment + starred variants, the matrix family,
  `subequations`, inline `$…$` / `\(…\)` / `\[…\]`; ~450 symbols/operators, full Greek,
  `\mathbb`/`\mathcal`/`\mathfrak`/`\mathscr`, accents, extensible arrows, and layout
  primitives. Hand-rolled — not delegated to an external math engine.
- **Tables** — `tabular`/`array`/`tabularray`/`tabularx` with full column specs, booktabs
  rules, `\multicolumn`/`\multirow`/`\makecell`.
- **Figures & floats** — `figure`/`table`/`algorithm`, `wrapfigure` (degrades to a standard
  float), `\includegraphics` + `\caption` + `\label`, multi-caption subfigure grids via
  `#subpar.grid`.
- **Theorems** — built-in kinds plus user kinds harvested from `\newtheorem`/`\newtcolorbox`/
  `\newmdenv`.
- **Code listings** — `lstlisting`/`verbatim`/`minted`/`\verb` → `#raw`.
- **References & citations** — `\label`/`\ref`/`\eqref`/`\cref`/`\Cref`; natbib/biblatex
  `\cite`/`\citet`/`\citep`, `.bib` preprocessing for Hayagriva, `.bbl` fallback, and
  `\bibliographystyle` mapping.
- **Custom macros** — `\newcommand`/`\renewcommand`/`\def`/`\newcommandx`/`\newif`,
  pre-scanned from every `.tex`/`.sty`/`.cls` in the project tree before conversion.

Anything else produces a structured warning categorised as `unsupported_command`,
`unsupported_environment`, `drop_only`, `unknown_package`, `tikz`, `custom_macro`,
`parse_error`, `ambiguous_math`, or `needs_manual_review`.

## Install

The `byetex` binary is the same across channels; the Claude Code plugin (skills +
MCP server) is a separate artifact that needs the binary on PATH.

```bash
# 1. Claude Code plugin — bundles the skills + auto-registers the MCP server.
claude plugin marketplace add zeyuyang42/ByeTex
claude plugin install byetex@byetex

# 2. Install script — prebuilt binary → ~/.local/bin.
curl -fsSL https://raw.githubusercontent.com/zeyuyang42/ByeTex/main/install.sh | sh

# 3. cargo (needs Rust 1.84+; --features mcp adds `byetex serve`).
cargo install byetex --features mcp

# 4. Homebrew.
brew install zeyuyang42/byetex/byetex
```

Pre-built binaries are published with each release for `x86_64`/`aarch64` Linux
(musl) and macOS, plus `x86_64` Windows; each archive bundles the `byetex` binary
and the `skills/` directory. See [`packaging/README.md`](packaging/README.md) for
all four channels and [`docs/plugin-setup.md`](docs/plugin-setup.md) for the
plugin (Claude Code / Cursor).

## CLI

The `byetex` binary has seven subcommands. `convert` is the workhorse; `diagnose` is the
headline path when the goal is "make it compile".

```bash
# Convert a LaTeX project FOLDER (recommended for real papers): auto-detects the entry
# .tex, pre-scans every .tex/.sty/.cls for \newcommand/\def, then converts.
byetex convert ./paper-source
#   → paper-source.typ, paper-source.warnings.json, paper-source.agent_brief.md

# Single file, custom output, skip the brief, or fold a real compile log into the brief:
byetex convert paper.tex
byetex convert paper.tex --output /tmp/out.typ
byetex convert paper.tex --no-brief
byetex convert paper.tex --compile          # also runs typst; equivalent to `agent-brief`

# Compile-repair loop: map each typst error back to its LaTeX fragment + repair skill.
byetex diagnose paper.tex                    # add --project (or pass a dir) for multi-file papers
#   → paper.typ + paper.diagnostics.json

# Stage-0 input oracle: is the INPUT LaTeX itself valid (compiled with tectonic)?
# Distinguishes "the input is broken" from "ByeTex has a bug". Skips cleanly if
# tectonic isn't installed.
byetex doctor paper.tex                       # --strict to fail hard; --full to also check the .typ

# Bundled repair skills (start with byetex-getting-started):
byetex skills list
byetex skills read byetex-repair-loop

# Run as an MCP server over stdio (requires --features mcp at build time):
byetex serve

# Regression corpus over the synthetic test corpus:
byetex corpus run --dir tests/corpus/

# Inspect the warnings:
cat paper.warnings.json | jq '.[].category.kind' | sort | uniq -c
```

`byetex agent-brief <input>` is a documented shorthand for `convert --compile`: it runs
`typst compile` and folds the real log into the brief (`--no-compile` to skip).

### Project mode

For real-world LaTeX projects with figures, bibliography files, and `\input` sub-files,
`--project` produces a self-contained Typst project directory that compiles end-to-end.
The input can be the entry `.tex` file or the project folder; for arXiv tarballs, point at
the unpacked folder and ByeTex picks the entry and harvests all sibling `.sty`/`.cls`
macros before converting.

```bash
byetex convert ./paper-source --project
# Writes paper-source.typst-project/ containing:
#   main.typ        — the converted Typst body
#   fig/foo.pdf     — asset files copied from the source project
#   refs.bib        — bibliography copied as-is (Typst reads it natively)
#   typst.toml      — optional manifest for known document classes (skip with --no-toml)
#   warnings.json   — structured warnings sidecar
#   agent_brief.md  — portable Markdown brief (skip with --no-brief)

byetex convert ./paper-source --project --project-out /tmp/my-project --force
typst compile /tmp/my-project/main.typ
```

## Output contract

`byetex convert` always writes, next to the input:

- `<stem>.typ` — the converted Typst document.
- `<stem>.warnings.json` — an array of warnings, **always written** (even if empty) so
  downstream tools can rely on the file existing.
- `<stem>.agent_brief.md` — a portable Markdown brief for LLM patching (skip with `--no-brief`).

`byetex diagnose` additionally writes `<stem>.diagnostics.json` — the content-anchored map
of each `typst compile` error to its LaTeX fragment and repair skill.

The `warnings.json` schema is fully documented at
[`docs/warnings.schema.json`](docs/warnings.schema.json) and locked by a regression test
(`crates/byetex-core/tests/warnings_schema.rs`). A representative warning:

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

`severity` is `info | warning | error`. The exit code of `byetex convert` is **always 0**
when conversion succeeds — even with warnings — so callers inspect the sidecar rather than
the exit code. (`byetex doctor --strict` is the one exception: it returns non-zero on a
broken input.)

## For AI agents

**Start with [`AGENTS.md`](AGENTS.md)** — the cold-start orientation for the repair loop.
[`docs/for-agents.md`](docs/for-agents.md) is the deeper technical reference. The short
version:

1. `byetex convert input.tex` is non-destructive and idempotent. Read `input.warnings.json`
   — empty means a clean conversion.
2. Each warning's `suggested_skill` points to one of the **12 bundled skills** in `skills/`
   that documents how to resolve that category. Reach them via `byetex skills read <name>`,
   by opening `skills/<name>.md`, or over MCP with `read_skill`.
3. When the `.typ` doesn't compile, `byetex diagnose input.tex` maps each typst error back
   to its LaTeX fragment + repair skill — the **compile-repair loop**. Edit the `.typ`,
   re-run `typst compile`, repeat.
4. To grade *visual* fidelity, the `byetex-visual-grading` skill drives a vision agent
   against [`docs/fidelity-rubric.md`](docs/fidelity-rubric.md) — the **visual-fidelity loop**.
5. For interactive use, `byetex serve` exposes the converter, the repair loop, and skills as
   **seven MCP tools**: `convert`, `convert_file`, `convert_fragment`, `convert_project`,
   `diagnose`, `list_skills`, `read_skill`.

## Project layout

New contributors: start with [`docs/architecture.md`](docs/architecture.md) for the code
map and a bird's-eye view of how a conversion flows through the crates.

```
ByeTex/
├── crates/
│   ├── byetex-core/        # the library — pure, no I/O
│   │   ├── src/
│   │   │   ├── parser.rs        # tree-sitter-latex frontend
│   │   │   ├── emit.rs          # the Emitter forward walk
│   │   │   ├── emit/            # 15 specialized emitters (math, tables, figures, …)
│   │   │   ├── style_profile.rs # per-class fidelity (title/abstract/headings/cites)
│   │   │   ├── project.rs       # project mode: plan + materialize
│   │   │   ├── diagnose.rs      # compile-error → LaTeX-fragment mapping
│   │   │   └── skills.rs        # skills embedded at build time
│   │   └── vendor/             # vendored tree-sitter-latex (MIT, Patrick Förster 2021)
│   ├── byetex-cli/         # the `byetex` binary (all filesystem/process I/O)
│   └── byetex-mcp/         # rmcp-backed MCP server (feature: mcp)
├── corpus/                 # arXiv regression corpus (manifest.json; payloads gitignored)
├── scripts/                # corpus_sweep.sh, acceptance.sh, visual_test.py, … (see scripts/README.md)
├── skills/                 # 12 bundled Markdown repair skills (+ INDEX.md)
├── tests/                  # corpus/ fixtures/ visual/ (outputs gitignored)
├── vendor/katex/           # KaTeX submodule — math-coverage TEST oracle, not a runtime dep
└── docs/
    ├── architecture.md         # the code map — start here
    ├── conversion-logic.md     # emitter behavior in prose
    ├── for-agents.md           # the agent contract
    ├── fidelity-rubric.md      # the visual-grading oracle
    ├── fidelity-backlog.md     # ranked fidelity issues from the vision audit
    ├── scorecard.md            # corpus quality history (gate + driver)
    └── warnings.schema.json
```

## Status

**Compile-rate — the gate.** **59/59** ByeTex-attributable arXiv papers compile (100%, 0
`BYETEX_FAIL`), tracked in
[`scripts/acceptance_baseline.json`](scripts/acceptance_baseline.json). The acceptance gate
(`scripts/acceptance.sh`) blocks any merge that regresses a known-passing paper.

**Visual fidelity — the driver.** Corpus composite fidelity score **0.821**, graded against
[`docs/fidelity-rubric.md`](docs/fidelity-rubric.md). The fidelity gate
(`scripts/fidelity_gate.sh`, baseline
[`scripts/fidelity_baseline.json`](scripts/fidelity_baseline.json)) flags render regressions;
remaining gaps are tracked and ranked in
[`docs/fidelity-backlog.md`](docs/fidelity-backlog.md). Full history: [`docs/scorecard.md`](docs/scorecard.md).

The supported subset grows incrementally; compile-rate is held at its ceiling while each
release pushes fidelity.

**Synthetic snippet corpus (secondary).** A separate set of synthetic doc snippets tracks
coverage *breadth* (a different measure from the arXiv compile gate above):

<!-- corpus-summary:start -->
_Last updated: 2026-06-12 (commit 7627f0c)_

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

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE) — your choice.

Vendored and third-party dependency licenses are documented in [NOTICE](NOTICE). The only
vendored source is `crates/byetex-core/vendor/tree-sitter-latex/` (MIT, Patrick Förster
2021). All 150 transitive cargo dependencies are MIT, Apache-2.0, Unlicense, or Zlib.
