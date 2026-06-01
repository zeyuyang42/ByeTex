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
# Primary (compile-rate, with failure attribution):
./scripts/corpus_sweep.sh --with-oracle

# Secondary (structural fidelity on the pinned set):
uv run --with requests --with Pillow python scripts/visual_test.py
```

The sweep needs the corpus payloads (`uv run --with requests python scripts/corpus_harvest.py`)
and `typst` + `tectonic` on PATH.

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

### Secondary: structural fidelity

Not yet captured as a committed number. `scripts/visual_test.py` already computes `word_recall`,
`heading_recall`, `page_ratio`, and an AI visual grade per paper; the next scorecard update should
record these for the pinned set so fidelity becomes a tracked driver (last informal reading:
~1/5 pinned papers structurally OK — the real headroom).
