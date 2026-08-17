# Layout-tier noise floors — measured 2026-08-17

What a layout property must clear before it can fail a build, and how far the
corpus currently sits from that line.

Produced by:

```bash
uv run --with pymupdf python scripts/layout_loop.py floors \
    --index tests/visual/index.json --synthetic --repeat 8 \
    --emit scripts/layout_floors.json
```

`scripts/layout_floors.json` is **frozen**: it changes only by explicit PR, never
as a side effect of a run. An auto-updating floor tracks the regression and is
worse than no floor at all.

---

## The rule

> **Corpus percentiles must never become thresholds.**

50 of 65 corpus papers already drift on leading. A corpus percentile therefore
measures ByeTex's *systematic bias*, and adopting it as a tolerance would encode
the bug as normal — the gate would pass precisely *because* everything is
broken. The corpus distribution is a **workload description**. It is reported
here beside the floors so the distance between them stays visible, and for no
other purpose.

A tolerance answers a different question: *is this difference beyond what the
instrument and legitimate engine differences can produce?*

---

## The three measured sources

### `self_floor` — the precondition

Each `truth.pdf` profiled against **itself**. Identical input must give identical
output, or nothing downstream means anything.

```
65 papers profiled; all exactly 1.0 ✓
```

This is a gate on the whole exercise, not a tolerance. Had any paper come back
non-1.0, extraction would be nondeterministic and every number in this document
void. The tool exits 1 in that case rather than reporting floors.

### `repeat_floor` — the instrument's own noise

The same generated `main.typ` compiled a second time and re-profiled. Isolates
typst's determinism plus ours from the converter's behaviour.

```
8 papers recompiled; identical on every property ✓
```

**Zero on every property.** The instrument contributes no noise, so every floor
below is due entirely to legitimate engine differences.

### `legit_floor` — the real lower bound

The largest per-property deviation across the synthetic LEGIT variants —
differences a converter must be *forgiven* for: a different font family, ragged
vs justified, half-point body-size drift, hyphenation policy, bibliography label
style. A threshold below this fires on correct output, which is the fastest way
to get a gate switched off.

---

## Results

`threshold = max(repeat_floor, legit_floor) × 1.5`

| property | repeat | legit | **proposed** | corpus p50 | fail @ thr |
|---|---:|---:|---:|---:|---:|
| `layout_page_trim_ratio` | 0.0000 | 0.0000 | **0.0050**\* | 0.0000 | **8** ✅ |
| `layout_body_font_ratio` | 0.0000 | 0.0500 | **0.0750** | 0.0024 | **10** ✅ |
| `layout_fontsize_tier_emd` | 0.0000 | 0.4032 | 0.6048 | 0.2886 | 13 |
| `layout_ink_ratio` | 0.0000 | 0.1201 | 0.1801 | 0.1421 | 27 |
| `layout_top_margin_ratio` | 0.0000 | 0.0319 | 0.0478 | 0.1311 | 51 |
| `layout_left_margin_ratio` | 0.0000 | 0.0000 | 0.0050\* | 0.1755 | 56 |
| `layout_leading_ratio` | 0.0000 | 0.0458 | 0.0687 | 0.1782 | 56 |
| `layout_right_margin_ratio` | 0.0000 | 0.0202 | 0.0303 | 0.2217 | 58 |
| `layout_lines_per_page_ratio` | 0.0000 | 0.0455 | 0.0682 | 0.2658 | 60 |
| `layout_small_tier_share_delta` | 0.0000 | 0.0012 | 0.0050\* | 0.0973 | 60 |
| `layout_text_width_ratio` | 0.0000 | 0.0030 | 0.0050\* | 0.0449 | 62 |

\* threshold is the **0.005 resolution floor**, not measured evidence: no legit
variant moves that property at all, so `max(repeat, legit) × 1.5` would be
exactly `0.0` and fire on PDF coordinate rounding. This is the one number here
not derived from measurement, and where it binds the JSON records
`threshold_source: "resolution_floor"`.

`layout_column_mismatch_frac` is **absent by design**. Its detector still
disagrees with itself on ~13% of pages, so a floor measured for it would be a
floor for the detector's noise rather than the converter's drift.

---

## What this says

**Two properties are promotable now** — at most ~10 papers fail, which is a
backlog rather than a wall:

- `layout_body_font_ratio` — 10 papers, and its threshold is **fully measured**
  (legit floor 0.05 from the half-point size-drift variant, ×1.5). The strongest
  candidate: the number rests on evidence rather than on the resolution floor.
- `layout_page_trim_ratio` — 8 papers, but its threshold is the resolution floor.
  Defensible (page size genuinely should not change) yet weaker as a precedent.

**Everything else fails on 27–62 of 65 papers.** That is not a threshold problem.
The corpus really does drift that much on margins, leading, line density and text
width — which is the finding this whole tier was built to surface, and it now
has numbers.

**A speculation corrected by measurement.** After the column detector was
disqualified, `layout_text_width_ratio` looked like the natural first gate: it is
independent of the detector and independently corroborated. Measured, **62 of 65
papers fail it**. It is the *most* drifted property in the corpus, not the safest
to gate. The plan's original choice and its obvious replacement were both wrong,
and only measurement said so.

---

## Promotion checklist

A property may be promoted into `--gate-layout` only when all four hold:

- [x] **(a)** no synthetic legit variant trips the proposed threshold — by
      construction, the threshold is `1.5×` the worst legit deviation
- [x] **(b)** `repeat_floor` ≈ 0 — measured exactly 0 on every property
- [x] **(c)** at most ~10 corpus papers fail — true for `layout_body_font_ratio`
      (10) and `layout_page_trim_ratio` (8) only
- [ ] **(d)** each of those papers **manually confirmed as real drift**

**(d) is not done, and it is the step that separates a gate from noise.** It is
the one thing `layout_loop.py floors` cannot establish: it can show that a number
is defensible, never that the papers failing it are genuinely broken. Ten papers
is a short enough list to check by eye, and that check is the next step before
any promotion PR.

---

## Environment

Floors are tool-version dependent — a typst upgrade shifts default leading and
can silently widen the true legit floor past a promoted gate. Recorded in the
JSON:

- `typst_version` and `pymupdf_version` at measurement time
- `safety_factor`, `self_floor_ok`, and the per-variant legit deviations

Re-measure after any typst or pymupdf upgrade, and treat a changed floor as a
finding rather than a formality.
