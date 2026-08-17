#!/usr/bin/env python3
# requires: none
"""Unit tests for the fidelity-gate comparator (scripts/fidelity_check.py).

Run: python3 scripts/tests/fidelity_check_test.py

Focus: the corpus `fidelity_score` is a weighted mean over the papers a run
measured, so comparing a PARTIAL run (`fidelity_gate.sh --papers a b c`) against
a whole-corpus baseline score compares two different populations. A targeted
subset of below-average papers then fails the gate no matter what the change
did. The per-paper checks are the meaningful signal for a subset run and already
skip papers absent from the run.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import fidelity_check as fc  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


def paper(word_recall, structure_ok=True):
    return {
        "word_recall": word_recall,
        "heading_recall": 1.0,
        "mean_ssim": 0.5,
        "page_ratio": 1.0,
        "status": "ok",
        "structure_ok": structure_ok,
    }


BASELINE = {
    "fidelity_score": 0.827,
    "papers": {"a": paper(0.95), "b": paper(0.90), "c": paper(0.80)},
}


def run(current):
    return fc.evaluate(current, BASELINE, score_tol=0.02, recall_tol=0.05)


# ── a partial run must not be gated on the corpus score ──────────────────────
partial = {"fidelity_score": 0.794, "papers": {"c": paper(0.80)}}
regs, _ = run(partial)
check(
    not any("fidelity_score" in r for r in regs),
    "partial run: corpus score is NOT compared (different populations)",
)
check(regs == [], f"partial run with unchanged per-paper metrics is clean, got {regs}")

# ── but a real per-paper regression in that same partial run still fails ─────
partial_bad = {"fidelity_score": 0.794, "papers": {"c": paper(0.70)}}
regs, _ = run(partial_bad)
check(
    any("word_recall" in r for r in regs),
    "partial run: a per-paper word_recall drop still registers",
)

# ── a FULL run is still gated on the corpus score ────────────────────────────
full_bad = {
    "fidelity_score": 0.700,
    "papers": {"a": paper(0.95), "b": paper(0.90), "c": paper(0.80)},
}
regs, _ = run(full_bad)
check(
    any("fidelity_score" in r for r in regs),
    "full run: a corpus-score drop still registers",
)

full_good = {
    "fidelity_score": 0.900,
    "papers": {"a": paper(0.95), "b": paper(0.90), "c": paper(0.80)},
}
_, imps = run(full_good)
check(
    any("fidelity_score" in i for i in imps),
    "full run: a corpus-score rise is still reported as an improvement",
)

# ── a run covering every MEASURED baseline paper counts as full even when the
#    baseline also holds unmeasured (truth_render_failed) entries ─────────────
baseline_with_unmeasured = {
    "fidelity_score": 0.827,
    "papers": {
        "a": paper(0.95),
        "b": paper(0.90),
        "c": paper(0.80),
        "u": {"word_recall": None, "status": "truth_render_failed"},
    },
}
regs, _ = fc.evaluate(full_bad, baseline_with_unmeasured, 0.02, 0.05)
check(
    any("fidelity_score" in r for r in regs),
    "unmeasured baseline papers don't make a full run look partial",
)

# ── the baseline's VINTAGE must be visible ───────────────────────────────────
# index.json accumulates and the gate measures 5 papers by default, so a
# committed baseline mixes measurements taken at different code versions. That
# is how two papers sat in the baseline as `truth_render_failed` while both had
# in fact been failing to COMPILE since before the baseline file was written.
mixed = {"papers": {
    "a": {"byetex_version": "byetex 0.7.3"},
    "b": {"byetex_version": "byetex 0.7.3"},
    "c": {"byetex_version": "byetex 0.6.1"},
    "d": {},  # predates stamping
}}
v = fc.baseline_vintages(mixed)
check(len(v) == 3, f"baseline_vintages counts the distinct versions present, got {list(v)}")
check(v.get("byetex 0.7.3") == ["a", "b"], f"...and names the papers per version, got {v}")
check(any("pre-dates" in k for k in v),
      f"an unstamped entry is reported as unknown vintage, not silently grouped, got {list(v)}")
check(len(fc.baseline_vintages({"papers": {"a": {"byetex_version": "x"}}})) == 1,
      "a single-vintage baseline reports exactly one version (no false alarm)")

# ── a PARTIAL run must NAME the baseline papers it did not measure ───────────
# The gate's default is the 5 pinned papers while the baseline holds 71, so the
# documented pre-release command checks 5/71 and skips the corpus-score
# comparison entirely (covers_whole_corpus is false). That is defensible only if
# it is IMPOSSIBLE to miss: a gate that silently covers 7% of the corpus reads
# exactly like a gate that covers all of it.
unchecked = fc.unchecked_papers(partial, BASELINE)
check(unchecked == ["a", "b"],
      f"a partial run names the baseline papers it did NOT measure, got {unchecked}")
check(fc.unchecked_papers(full_good, BASELINE) == [],
      "a full run has nothing unchecked")
check(fc.unchecked_papers(full_bad, baseline_with_unmeasured) == [],
      "a paper with no truth render is not counted as 'unchecked' — it is unmeasurable, "
      "which the UNMEASURED block reports separately")
check(fc.covers_whole_corpus(partial, BASELINE) is False
      and fc.covers_whole_corpus(full_good, BASELINE) is True,
      "covers_whole_corpus stays consistent with unchecked_papers")

# ── the layout tier reports, it does not gate (until told to) ────────────────
# "Driver first, gate later": a new tier that can fail a build on day one gets
# `|| true`-d within a week (Chromium's documented layout-dump failure). So
# nothing new can fail anything until a property is named in --gate-layout, one
# at a time, each with measured floors behind it.
LB = {"papers": {
    "a": {**paper(0.95), "anchor_recall": 0.90, "ordered_recall": 0.90,
          "layout_text_width_ratio": 1.00},
}}
LC = {"papers": {
    "a": {**paper(0.95), "anchor_recall": 0.40, "ordered_recall": 0.40,
          "layout_text_width_ratio": 1.42},
}}
regs, notices, imps = fc.evaluate_layout(LC, LB, gated=set(), tols=fc.LAYOUT_TOLERANCES)
check(regs == [], f"with nothing gated, a collapsed layout metric CANNOT fail the build, got {regs}")
check(len(notices) >= 3, f"...but every breach is still REPORTED, got {notices}")

regs, notices, imps = fc.evaluate_layout(
    LC, LB, gated={"layout_text_width_ratio"}, tols=fc.LAYOUT_TOLERANCES)
check(any("layout_text_width_ratio" in r for r in regs),
      f"a PROMOTED property does fail the build, got {regs}")
check(not any("anchor_recall" in r for r in regs),
      "promoting one property does not promote the others (per-property, not a global switch)")
check(any("anchor_recall" in n for n in notices),
      "an un-promoted property stays a notice")

# None-safety: until a baseline is re-emitted every new field is absent.
old_style = {"papers": {"a": paper(0.95)}}
regs, notices, imps = fc.evaluate_layout(LC, old_style, gated=set(fc.LAYOUT_TOLERANCES),
                                         tols=fc.LAYOUT_TOLERANCES)
check(regs == [] and notices == [],
      f"a baseline predating the tier is SKIPPED, not reported as a mass regression, got {regs or notices}")

# `toward_one`: an improvement is |x-1| shrinking, so an overshoot is still drift.
base_w = {"papers": {"a": {**paper(0.95), "layout_text_width_ratio": 1.40}}}
cur_ok = {"papers": {"a": {**paper(0.95), "layout_text_width_ratio": 1.01}}}
_, _, imps = fc.evaluate_layout(cur_ok, base_w, gated=set(), tols=fc.LAYOUT_TOLERANCES)
check(any("layout_text_width_ratio" in i for i in imps),
      f"moving a ratio TOWARD 1.0 is an improvement, got {imps}")
cur_over = {"papers": {"a": {**paper(0.95), "layout_text_width_ratio": 0.55}}}
_, notices, _ = fc.evaluate_layout(cur_over, base_w, gated=set(), tols=fc.LAYOUT_TOLERANCES)
check(any("layout_text_width_ratio" in n for n in notices),
      "overshooting past 1.0 into the opposite error is still drift, not a win")

# A property whose DETECTOR is known-broken must not be reportable or gateable.
# The column detector reads 6 columns on single-column pages of real papers and
# fired on 58/65 of the corpus — one broken detector, not 58 broken papers.
# Reporting it would bury the trustworthy properties under noise.
check("layout_column_mismatch_frac" not in fc.LAYOUT_TOLERANCES,
      "the column detector is excluded until it distinguishes a gutter from a sparse page")
check("layout_column_mismatch_frac" in fc.BASELINE_FIELDS_LEGACY
      or "layout_column_mismatch_frac" not in fc.REPORTED_FIELDS,
      "...and is therefore not in REPORTED_FIELDS")

check(set(fc.GATED_FIELDS) <= set(fc.BASELINE_FIELDS)
      and "anchor_recall" in fc.BASELINE_FIELDS,
      "BASELINE_FIELDS persists both the gated set and the reported tier")

# ── promotion: measured floors + a KNOWNBAD list ─────────────────────────────
# A gate that starts RED gets `|| true`-d within a week. So the first promoted
# property ships with the papers that already fail it listed as known-bad: the
# gate is green on day one and fires only on NEW breakage. That is the Chromium
# lesson and the SILE `KNOWNBAD` model.
FLOORS = {
    "properties": {
        "layout_body_font_ratio": {"proposed_threshold": 0.075},
        "layout_leading_ratio": {"proposed_threshold": 0.0687},
    },
    "known_bad": {"layout_body_font_ratio": ["old1", "old2"]},
}
tol = fc.tolerances_from_floors(FLOORS, fc.LAYOUT_TOLERANCES)
check(tol["layout_body_font_ratio"] == ("toward_one", 0.075),
      f"a MEASURED threshold overrides the provisional default, got {tol['layout_body_font_ratio']}")
check(tol["anchor_recall"] == fc.LAYOUT_TOLERANCES["anchor_recall"],
      "a property with no measured floor keeps its provisional default")

BAD = {"papers": {
    "old1": {**paper(0.95), "layout_body_font_ratio": 1.40},   # already broken
    "old2": {**paper(0.95), "layout_body_font_ratio": 0.80},   # already broken
    "fresh": {**paper(0.95), "layout_body_font_ratio": 1.00},  # currently fine
}}
# Day one: old1/old2 sit past the ABSOLUTE floor, so the only thing keeping the
# gate green is the known_bad list.
regs, _, _ = fc.evaluate_layout(BAD, BAD, gated={"layout_body_font_ratio"}, tols=tol,
                                known_bad=FLOORS["known_bad"])
check(regs == [], f"the gate is GREEN on day one — known-bad papers do not fail it, got {regs}")

# ...and THIS is the control that makes the assertion above mean anything. It used
# to pass with `known_bad={}` as well: the tolerance was applied as a delta, so
# with baseline == current no branch could fire and the test never exercised the
# exclusion it named. Drop the amnesty and the same call must go RED.
regs_no_amnesty, _, _ = fc.evaluate_layout(BAD, BAD, gated={"layout_body_font_ratio"},
                                           tols=tol, known_bad={})
check(sorted(r.split(":")[0] for r in regs_no_amnesty) == ["old1", "old2"],
      "without known_bad those same papers DO fail — the exclusion is what makes "
      f"day one green, got {regs_no_amnesty}")

# The absolute floor also has to fire on a paper that never moves: parked just
# past the threshold forever is still drift, and a delta rule could never see it.
PARKED = {"papers": {"parked": {**paper(0.95), "layout_body_font_ratio": 1.20}}}
regs_parked, _, _ = fc.evaluate_layout(PARKED, PARKED, gated={"layout_body_font_ratio"},
                                       tols=tol, known_bad={})
check(any("parked" in r for r in regs_parked),
      f"a paper parked past the floor fires even when it did not change, got {regs_parked}")

# A NEW paper breaking the same property must fail.
BROKE = json.loads(json.dumps(BAD))
BROKE["papers"]["fresh"]["layout_body_font_ratio"] = 1.40
regs, _, _ = fc.evaluate_layout(BROKE, BAD, gated={"layout_body_font_ratio"}, tols=tol,
                                known_bad=FLOORS["known_bad"])
check(any("fresh" in r for r in regs),
      f"a NEW paper breaking a gated property DOES fail the build, got {regs}")
check(not any("old1" in r or "old2" in r for r in regs),
      "...while the known-bad papers stay excluded")

# A known-bad paper getting WORSE still must not fail — it is already on the list,
# and re-reporting it would make the gate noisy without adding information.
WORSE = json.loads(json.dumps(BAD))
WORSE["papers"]["old1"]["layout_body_font_ratio"] = 2.00
regs, _, notices = fc.evaluate_layout(WORSE, BAD, gated={"layout_body_font_ratio"}, tols=tol,
                                      known_bad=FLOORS["known_bad"])
check(regs == [], f"a known-bad paper degrading further does not fail the gate, got {regs}")

# But a known-bad paper that gets FIXED must be reported, or the list rots.
FIXED = json.loads(json.dumps(BAD))
FIXED["papers"]["old1"]["layout_body_font_ratio"] = 1.00
_, _, imps = fc.evaluate_layout(FIXED, BAD, gated={"layout_body_font_ratio"}, tols=tol,
                                known_bad=FLOORS["known_bad"])
check(any("old1" in i for i in imps),
      f"a known-bad paper that is FIXED is reported so the list can shrink, got {imps}")

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
