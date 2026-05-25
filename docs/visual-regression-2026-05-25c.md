# Visual Regression — 2026-05-25 (third pass)

## Context

This document is a **commit-derived changelog**, not a fresh `visual_test.py` run.
It enumerates all work merged into `main` since
[`2026-05-25b.md`](visual-regression-2026-05-25b.md) (which closed after PR #42,
with the 5-paper canonical suite at 1/5 `typst_ok` and the 26-paper corpus at 7/26).

No rerun of `scripts/visual_test.py` was performed for this report.
The updated measured corpus pass-rate will appear in the next report, after Bug #41
lands and there is a clean `main` to run against.

## Bugs closed since 2026-05-25b

| # | Description | Commit(s) | Area |
|---|---|---|---|
| **#30** | math-env `\label{...}` flushed outside `$...$` instead of inline | `86f8b7f`, `44a3982` | math emitter |
| **#31** | `\\` row-break inside a nested matrix cell leaked a stray `\` byte | `c690fb9`, `a8481d3` | matrix / row-break |
| **#33** | bare-letter subscript (`_h`) fused with the following letter (`j` → `hj`) | `5b9ec0a`, `61f3a4e` | math subscript |
| **#35** | section title + `\label` dropped when tree-sitter mis-parses the heading | `7f7ad5a`, `3436f3f` | section / tree-sitter |
| **#36** | `;` in math function-call arguments emitted unescaped; follow-up used wrong escape form | `0924cf9`, `983cc58`, `de23e5c`, `effd6b9` | math escape |
| **#37** | `\includegraphics` paths without extension not resolved; figures with no graphic emitted `image("???")` placeholder that fails compile | `a4dd1ca`, `e2f4d3f` | asset resolution |
| **#38** | `tabular` row-split at `\\` broke when `\\` appeared inside an escape sequence | `26a6aae`, `5e738f2` | tabular |
| **#39** | theorem `kind:` field rejected by Typst when it contained `:` or non-ASCII chars; math-style switches and wrapper `\newcommand` macros emitted verbatim instead of dropped | `7d65e6f`, `73385ee`, `f0f2a64`, `dd7a4fe` | theorem / markup |
| **#40** | `.bib` files passed verbatim to Typst's strict BibLaTeX parser caused compile failures on common fields (`date`, `langid`, `keywords`) | `b47581f`, `dd7a4fe` | bibliography |
| **#41** | arXiv papers shipping only a `.bbl` (no `.bib` source) had no bibliography; now probes `base_dir` for a lone `.bbl` and inlines it through `emit_thebibliography`; also sanitizes `\label` keys that contain `\`, `/`, or `^` | `5ad4786`, `44553ef` | bibliography / label |

### Other landed work

| Commit | Description |
|---|---|
| `778da75` | cli: slim `agent_brief.md` to a pointer-only file (~1 KB; was ~90 KB) |
| `abdd641` | ci: fix `bytetex`→`byetex` typo in CI workflow + restore `cargo fmt` check |

## Deferred (still open)

| # | Description | Decision |
|---|---|---|
| **#32** | `\cite{key}` references to entries in dropped `.bib` files produce broken `@key` labels | Defer — requires BibTeX simulation (parsing surviving `.bib` to know which keys are defined). Roadmap item. |
| **#34** | Compiled papers have low `heading_recall` and shorter page count than the original PDF | Defer — fidelity track, separate from compile-blocker work. |

## Next steps

Rerun `scripts/visual_test.py` (26-paper corpus) once Bug #41 lands on `main` to
capture the updated `typst_ok` and `structure_ok` counts.
