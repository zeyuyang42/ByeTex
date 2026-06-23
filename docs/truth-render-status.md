# Truth-render status (corpus)

The fidelity DRIVER needs a *truth* PDF — the paper's original LaTeX rendered with tectonic.
Run `scripts/setup_truth_deps.sh` first (pinned biber + fonts). This file records the papers
whose truth does **not** render even with deps provisioned, so they're never mistaken for a
silent unmeasured "pass". Updated 2026-06-23 (health-check Phase 0a).

## Now rendering (promoted unmeasured → measured)
- **gh-dzwaneveld-tudelft-thesis** — was `truth_render_failed` (needed Roboto Slab + biber).
  Now renders; measured `word_recall=0.962`, `heading_recall=1.0`, but `page_ratio=0.50`
  (ByeTex 6pp vs truth 12pp) → `structure_failed`. The page gap is the Phase-4 cover/density work.

## Still `truth_render_failed` (recorded reason — not a ByeTex defect)
| Paper | Reason (tectonic, deps present) | Class |
|---|---|---|
| ctan-memoir | needs a pre-built `trims-example.pdf` (the memoir *manual*, not a normal doc) | input |
| gh-calpolycsc-thesis | `main.tex:204: Undefined control sequence` (source macro gap) | input |
| gh-fmarotta-kaobook | `kaobook.cls not found` (class file not vendored in source) | ingestion |
| gh-maurovm-thesis-template | font chain: Carlito ✓ then `Latin Modern Math cannot be found` | font |
| gh-pelegs-maths-book | `svg` package needs inkscape-built `tapir_svg-tex.pdf` | input |
| gh-sikatikenmogne-report | `subcaption` can't co-exist with `subfig` (source bug) | input |

`gh-maurovm` is the only remaining *font*-class failure (add the math font to
`setup_truth_deps.sh` to recover it); the rest are source/ingestion issues.

## Ingestion gate (Phase 0b)
`scripts/corpus_add_local.py` now renders the truth BEFORE accepting a paper and records
`truth_render_status` (`ok` | `failed` | `unverified`) in both `corpus/manifest.json` and the
paper's `00README.json`. A failed render REJECTS the paper (removes the half-added dir) unless
`--allow-no-truth` is passed — then it's accepted with `truth_render_status=failed` + the reason,
so it's never a silent unmeasured "pass". Run `scripts/setup_truth_deps.sh` first.

## Surfaced bug — acceptance gate blind spot (separate fix)
- **2605.31063** is in acceptance `known_pass` yet its ByeTex output **fails `typst compile`**
  on current `main` (`error: unexpected argument` at `main.typ:5244`). The acceptance sweep is
  not catching it (the gate-blindspot noted in the health check). This is a real converter bug,
  **out of scope for the truth-pipeline tick** — file it as its own Loop-A item. Deliberately
  NOT recorded as a fidelity regression here (it is unrelated to the truth-deps change).
