# Truth-render status (corpus)

The fidelity DRIVER needs a *truth* PDF — the paper's original LaTeX rendered with tectonic.
Run `scripts/setup_truth_deps.sh` first (pinned biber + fonts). This file records the papers
whose truth does **not** render even with deps provisioned, so they're never mistaken for a
silent unmeasured "pass". Updated 2026-09-01.

Since 2026-09-01 the harness reports these reasons *itself*:
`truth_render.summarize_tectonic_failure` reads the cause off tectonic's `error:` lines,
skipping its warnings and its wrapper lines. It used to scan all of stderr for a
font/package keyword, which matched warnings just as readily — so three of the rows below
were reported as a Carlito font-path notice, an `algorithm.sty` UTF-8 notice and a
`kpfonts` notice, none of which was the cause. The table was right; the harness was not,
and a misattributed failure is one nobody fixes.

## Now rendering (promoted unmeasured → measured)
- **gh-dzwaneveld-tudelft-thesis** — was `truth_render_failed` (needed Roboto Slab + biber).
  Now renders; measured `word_recall=0.962`, `heading_recall=1.0`, but `page_ratio=0.50`
  (ByeTex 6pp vs truth 12pp) → `structure_failed`. The page gap is the Phase-4 cover/density work.

## Still `truth_render_failed` (recorded reason — not a ByeTex defect)
| Paper | Reason (tectonic, deps present) | Class |
|---|---|---|
| ctan-memoir | needs a pre-built `trims-example.pdf` (the memoir *manual*, not a normal doc) | input |
| gh-calpolycsc-thesis | **changed 2026-09-01:** now `Found biblatex control file version 3.8, expected version 3.11` — the biber pinned by `setup_truth_deps.sh` has drifted out of sync with the biblatex in tectonic's current bundle. Was `main.tex:204: Undefined control sequence`. | deps |
| gh-fmarotta-kaobook | `kaobook.cls not found` (class file not vendored in source) | ingestion |
| gh-maurovm-thesis-template | font chain: Carlito ✓ then `Latin Modern Math cannot be found` | font |
| gh-pelegs-maths-book | `svg` package needs inkscape-built `tapir_svg-tex.pdf` | input |
| gh-sikatikenmogne-report | `subcaption` can't co-exist with `subfig` (source bug) | input |

`gh-maurovm` is the only remaining *font*-class failure (add the math font to
`setup_truth_deps.sh` to recover it). `gh-calpolycsc-thesis` is now a **deps** failure and
should be recoverable by re-pinning biber to match the bundle's biblatex — both are harness
work, not source bugs. The rest are source/ingestion issues.

**`gh-dzwaneveld-tudelft-thesis` renders, but flakily.** It was reported
`truth_render_failed` in the 2026-09-01 gate run and renders fine standalone (10.7s) with a
valid cached `truth.pdf` on disk. The run had concurrent tectonic invocations; treat a lone
failure on this paper as contention, not drift, and re-check before recording it.

## Ingestion gate (Phase 0b)
`scripts/corpus_add_local.py` now renders the truth BEFORE accepting a paper and records
`truth_render_status` (`ok` | `failed` | `unverified`) in both `corpus/manifest.json` and the
paper's `00README.json`. A failed render REJECTS the paper (removes the half-added dir) unless
`--allow-no-truth` is passed — then it's accepted with `truth_render_status=failed` + the reason,
so it's never a silent unmeasured "pass". Run `scripts/setup_truth_deps.sh` first.

## Surfaced bug — acceptance gate blind spot (FIXED)
- **2605.31063** — FIXED (PR fix/attach-comma, v0.6.5). Was in acceptance `known_pass` yet
  its ByeTex output **failed `typst compile`** (`error: unexpected argument` at `main.typ:5244`):
  an `\overset`-family construct with a COMMA in the over-text leaked the comma into the Typst
  `attach(base, t: script)` arg list, where it was read as a stray second positional argument.
  `emit_math_attach` now wraps a comma-bearing script in `#box[$ … $]` (contains the comma,
  no visible delimiters). The paper now compiles cleanly (70 pages).
