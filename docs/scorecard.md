# ByeTex Corpus Scorecard

The single authoritative measure of converter quality. Every change is judged against this.

## Decision rule

A change ships **iff**:
1. **Gate — compile-rate holds or improves.** The emitted `.typ` must compile with `typst compile`
   for at least as many corpus papers as the current baseline.
2. **No pinned regression.** None of the 5 pinned papers may go PASS → FAIL.
3. **Driver — structural fidelity holds or improves** on the pinned set (see *Secondary signals*).

Compile-rate is the **gate**; structural fidelity is the **driver**. Compile-rate is already near
its ceiling (23/25 ByeTex-attributable, see below), so most real quality headroom is in fidelity —
track both, never trade one for the other.

## How to reproduce

```bash
# Acceptance gate (Phase-3 oracle) — the regression check. Fails (exit 1) iff a
# known-passing paper regresses to BYETEX_FAIL. Run before merging:
BYETEX_BIN=target/release/byetex ./scripts/acceptance.sh

# Primary (compile-rate, with failure attribution):
./scripts/corpus_sweep.sh --with-oracle

# Secondary (structural fidelity on the pinned set):
uv run --with requests --with Pillow python scripts/visual_test.py
```

The sweep needs the corpus payloads (`uv run --with requests python scripts/corpus_harvest.py`)
and `typst` + `tectonic` on PATH.

**Acceptance gate (`scripts/acceptance.sh`).** Encodes the decision rule below as an
automated check on top of the tectonic round-trip: it runs `corpus_sweep.sh --with-oracle`
and compares the per-paper verdicts to `scripts/acceptance_baseline.json` (a committed
`known_pass` / `known_fail` partition). Compile-rate is the **hard gate** — a `known_pass`
paper that newly fails to compile exits non-zero; a fixed paper or a newly-harvested failing
paper is **reported** (update the baseline). Only papers whose payloads are present are
checked, so gitignored payloads never cause a false regression. Fidelity stays the **driver**
(measured by `visual_test.py`, not gated). When a fix flips a paper, promote it from
`known_fail` to `known_pass` in the baseline.

---

## Baseline — 2026-06-01 (commit 5fedfe0)

### Primary: compile-rate

Verbatim summary from `./scripts/corpus_sweep.sh --with-oracle`:

```
PASS: 23  BYETEX_FAIL: 2  INPUT_BROKEN: 1  UNATTRIBUTED: 0  SKIP: 2  TOTAL: 28
```

The 28 swept directories are the 26 corpus papers plus `inhouse/` and `online/`, which have no
harvested `source/00README.json` and are the 2 SKIPs. So of the **26 real papers**:

| Bucket | Count |
|---|---:|
| **PASS** (compiles) | **23 / 26** |
| BYETEX_FAIL (our output broke) | 2 |
| INPUT_BROKEN (source won't compile either) | 1 |

Excluding the source that is itself broken, **ByeTex-attributable compile-rate = 23/25 (92%)**.

**This supersedes the stale figures** in `docs/test-results-2026-05-23.md` (29% compile) and the
README "87% pass-rate" (which only measured *ran without parse error*, a weaker bar).

### Current failures

Errors below are quoted verbatim from the sweep log. Root causes are **not yet investigated** —
that is Phase 1/2 implementation work, deliberately out of scope for this scorecard PR.

| Paper | Verdict | `typst` error (verbatim) |
|---|---|---|
| `2605.22579` | BYETEX_FAIL | `label <icmlsymbolequal> does not exist in the document` |
| `2605.22814` | BYETEX_FAIL | `label <sec:coverage> does not exist in the document` |
| `2605.22821` | INPUT_BROKEN | `unclosed delimiter` (source itself won't compile — **not** a ByeTex bug, per the `byetex doctor` oracle) |

Both ByeTex-attributable failures surface as **missing-label / dangling-reference** errors
(a `@key` reference with no matching `<key>` anchor). Neither token appears literally in the
paper source, so both anchors are produced during conversion — the specific mechanism is a fix
target for a later phase, not a claim made here.

### Pinned regression set (must never go PASS → FAIL)

`2605.22820`, `2605.22776`, `2605.22557`, `2605.22159`, `2605.22507` — **all PASS** at baseline.

### Secondary: structural fidelity — committed baseline (2026-06-01)

Now a tracked number. `scripts/visual_test.py` computes deterministic structural metrics per
paper; Phase 2a added three that the set-based `word_recall`/`heading_recall` are **blind** to:
- **word_count_ratio** — typst/truth prose-token *count* (catches deletion <1.0 / duplication >1.0
  that set-recall misses).
- **heading_sequence_score** — longest in-order (LCS) heading match / truth headings (catches
  reorder/flatten that `heading_recall` ignores).
- **figure_ratio / table_ratio** — distinct `Figure N`/`Table N` caption counts, typst vs truth
  (catches dropped/spurious floats invisible to word & heading metrics).

**Baseline command** (offline, deterministic — tectonic renders the truth PDF, no network):
```
uv run --with requests --with Pillow --with numpy --with scikit-image \
  python scripts/visual_test.py --truth-source tectonic --no-truth-download \
  --rasterize-dpi 100
```

**Pinned-set baseline.** Only 3 of the 5 pinned papers are usable: tectonic cannot compile the
LaTeX of `2605.22557` (hypdvips/hyperref driver conflict) or `2605.22159` (undefined control
sequence) — `truth_render_failed`, a *truth-source* limit, not a ByeTex defect. Those two need the
arXiv canonical PDF (drop `--no-truth-download`) to be scored.

| paper | word_recall | word_count_ratio | heading_recall | heading_seq | figure_ratio | table_ratio | mean_ssim |
|---|---|---|---|---|---|---|---|
| 2605.22820 | 0.89 | 0.98 | 0.89 | 0.89 | 1.00 (6/6) | 1.00 (17/17) | 0.57 |
| 2605.22776 | 0.96 | 1.14 | 0.78 | 0.70 | **1.88 (8→15)** | **0.12 (8→1)** | 0.60 |
| 2605.22507 | 0.85 | 0.97 | 0.67 | 0.67 | 1.20 (10→12) | 1.00 (2/2) | 0.56 |

**What the new metrics immediately surfaced (the Phase-2b triage seed):** `2605.22776` looks fine
on the legacy metrics (word_recall 0.96) but is **dropping 7 of 8 tables (table_ratio 0.12) and
emitting ~7 spurious figures (figure_ratio 1.88)** — a major structural defect the set-based
metrics completely missed. `heading_seq` < `heading_recall` on 22776 also flags heading
reorder/flatten. These are the kind of defects Phase 2c will fix in slices.

Known gaps to address next: `page_ratio` is not yet persisted into `index.json` (shows null);
the tectonic-truth path covers 3/5 pinned papers; thresholds for the new metrics are not yet
*gated* (reported only) — gate them once more papers establish realistic cross-engine ranges.

### Update 2026-06-01 (Phase 2c D1 + D4) — corrected, source-anchored numbers

Two fixes changed the fidelity picture:
- **D1 (PR #146):** table floats now caption as "Table N" and `\input`-ed / `\\[len]` tabulars
  are recovered. 22776 emits 8 of 8 tables (was 1), 22817 9 of 9 (was 2).
- **D4 (this work):** the heading + float metrics were unreliable on math-heavy papers because
  truth headings/floats came from `pdftotext` of the rendered PDF, which lifts equation
  fragments in as bogus headings. Both sides are now **source-anchored**: truth headings/float
  counts from the project LaTeX (`\section`/`\begin{figure|table}`, all `\input`-ed files), and
  the typst side from byetex's own `.typ` `= H`/`== H` markers (not pdftotext). The metric now
  measures *converter* fidelity, not extraction noise.

Re-measured `heading_recall` (pdftotext baseline → source-anchored):

| paper | heading_recall (was → now) | note |
|---|---|---|
| 2605.22507 | 0.67 → **1.00** | was pure extraction noise |
| 2605.22584 | 0.30 → **0.94** | was pure extraction noise |
| 2605.22776 | 0.78 → **1.00** | + table_ratio 0.12 → **1.00**, figure_ratio 1.88 → **1.00** (D1) |
| 2605.22765 | 0.55 → **0.65** | a *real* remaining heading gap (now trustworthy) |
| 2605.22820 | 0.89 → **0.86** | flat — both sides clean; a real small gap |

The false-negative noise is gone. Remaining sub-1.0 numbers (22765 0.65, 22820 0.86) are now
**real heading-fidelity signals** — the next Phase-2c targets — rather than measurement
artifacts. The `truth_render_failed` 3/5-pinned coverage caveat above still applies (tectonic
can't compile those two sources).

### Update 2026-06-02 (figure cluster + metric polish)

**Figure-fidelity cluster shipped** (D1 tables, D5 subfigures, D6 `\input`-relative image
paths, D7 `\graphicspath`): every corpus paper that emitted 0 images now renders its figures
(22765 0→16, 22800 0→11, 22821 32→63, etc.).

**Two metric-accuracy fixes** removed the last extraction artifacts:
- `typ_headings` now also extracts byetex's `#heading(...)[Title]` form (starred/unnumbered
  sections). 22820 heading_recall **0.86 → 1.00** (the 4 back-matter sections were emitted all
  along, just not via `=` markers).
- `typ_float_counts` counts real image figures + `kind: table` from the `.typ`, excluding
  `#figure(kind: "equation"|"remark"|…)` blocks the PDF count mistook for figures. Spurious
  figure over-counts collapse to 1.00: 22820 1.33→1.00, 22765 2.10→1.00, 22728 2.40→1.00,
  22549 1.50→1.00.

Remaining **real** (now-trustworthy) signals for future slices: 22765 heading_recall 0.65;
22817 table_ratio 1.29 (emits more tables than the source has — possible spurious table);
22728 heading_recall 0.38 (dense-math paper). All modest.

### Update 2026-06-02 (compile-blocker sprint + layout density)

**Compile-rate: 33 → 40 / 45 ByeTeX-attributable (~89%)** on the 56-paper corpus. Eleven
fixes (#155–165) cleared distinct blockers: email/`@` escaping, math empty-base & font-decl
args, the `braket` package, table-cell math escaping, dual-bibliography & multi-line/paren bib
paths, dangling-ref backstop, `\sqrt\frac{}`, and emphasis whitespace. Remaining 5 are a
fragile tail (core math-tokenization + niche edge cases).

**Layout-density fidelity (the driver).** With content now correct (headings/floats/word-count
≈ 1.0), the dominant remaining gap was **page-count inflation**: byetex rendered ~1.13–1.21×
longer than the LaTeX truth. Two neutral-preamble fixes match LaTeX `article` density:
- default body size **11pt → 10pt** (LaTeX's `article` default when no size option is given);
- paragraph **`spacing: 0.65em`** (= line leading) so paragraph breaks are indent-only, not
  Typst's default ~1.2em inter-paragraph gap.

Result (tectonic-truth, `page_ratio` = typst_pages / truth_pages):

| paper | before | after |
|---|---|---|
| 2605.22820 | 1.17 (46→54) | **0.93** (46→43) |
| 2605.22776 | 1.13 | **0.91** |
| 2605.22507 | 1.21 | **0.97** |
| 2605.22765 | — | **0.96** |
| 2605.22584 | — | **1.00** |
| 2605.22817 | — | **1.08** |
| 2605.22779 (two-column) | 1.69 | 1.38 (still high) |

Mean `page_ratio` ~1.20 → **~1.03**; 6/7 sampled within [0.9, 1.1]. Compile-rate held at 40.

**Composite fidelity score now weights page density.** The single corpus-wide number is

> `0.35·word_recall + 0.25·heading_recall + 0.2·mean_ssim + 0.2·page_closeness`

where `page_closeness = min(r, 1/r)` for `r = typst_pages / truth_pages` — a symmetric [0,1]
measure that is 1.0 at an exact page-count match and penalizes running either longer **or**
shorter than the LaTeX truth (SSIM is too coarse to reward a global font-size shift, so the
layout work above was previously invisible to the headline number). `page_ratio` is persisted
per-paper in `index.json`; papers missing any metric (including a renderable page count) are
skipped, not scored zero. Sample (8 papers) composite after #166/#167: **0.809**, with
`page_closeness` 0.81–1.00. Two-column figure/equation sizing (22779 residual 1.23) is a
separate follow-up.

**Measurement gotcha:** `scripts/visual_test.py` builds/uses `REPO_ROOT/target/release/byetex`
and ignores `BYETEX_BIN` — run it **from the worktree** (with the corpus symlinked) to measure a
branch's binary, or the stale main-checkout binary silently masks the change.
