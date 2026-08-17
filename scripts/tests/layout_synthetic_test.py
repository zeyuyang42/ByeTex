#!/usr/bin/env python3
# requires: pymupdf
"""Ground-truth test for the layout tier, against synthesized drift.

Run: uv run --with pymupdf python scripts/tests/layout_synthetic_test.py
Skips cleanly (prints SKIP, exits 0) without typst / pdftotext / pymupdf.

Layout drift is the ONE thing in this harness that can be synthesized with known
ground truth: render the same body twice with exactly one property changed, and
you know what the answer must be. So unlike every other metric here, this one's
correctness is CHECKABLE rather than merely plausible. That is the entire reason
this fixture set exists.

Precedent for taking the controls seriously: scripts/fidelity_audit.py:163-233
records that 12 of 13 probes were false positives before controls were added,
and that two of the first-draft controls were themselves vacuous.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _layout_fixtures as fx  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import layout_metrics as lm  # noqa: E402

missing = fx.missing_tools()
if missing:
    print(f"SKIP: layout_synthetic — missing {', '.join(missing)}")
    sys.exit(0)

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


manifest = fx.load_manifest()
print(f"building {len(manifest['variants']) + 2} PDFs …")
pdfs = fx.build_all(manifest)
base = pdfs["base"]

RATIO_KEYS = [k for k in lm.compare_profiles([], []) if k.endswith("_ratio")]

# ═════════════════════════════════════════════════════════════════════════════
# controls 2 and 3 — the instrument, before any claim about the corpus
# ═════════════════════════════════════════════════════════════════════════════
print("\n── control 2: identity ──")
ident = fx.metrics_for(base, base)
bad = [f"{k}={ident[k]}" for k in RATIO_KEYS if ident.get(k) != 1.0]
check(not bad, f"a PDF compared against ITSELF is exactly 1.0 on every ratio {bad}")
check(lm.fired_classes(ident) == set(), f"identity fires nothing, got {lm.fired_classes(ident)}")
check(ident["ordered_recall"] == 1.0 and ident["ordered_precision"] == 1.0,
      "identity is 1.0 on the ordered stream too")

print("\n── control 3: recompile determinism ──")
# If compiling identical source twice does not give an identical profile, the
# instrument has its own noise and every floor below is measuring that noise.
rep = fx.metrics_for(base, pdfs["base2"])
bad = [f"{k}={rep[k]}" for k in RATIO_KEYS if rep.get(k) != 1.0]
check(not bad, f"the SAME source compiled twice profiles identically {bad}")
check(lm.fired_classes(rep) == set(), "a recompile fires nothing (repeat_floor is zero)")

# ═════════════════════════════════════════════════════════════════════════════
# the goldens
# ═════════════════════════════════════════════════════════════════════════════
print("\n── variant goldens ──")
results: dict[str, dict] = {}
for v in manifest["variants"]:
    name, kind = v["name"], v["kind"]
    m = fx.metrics_for(base, pdfs[name])
    results[name] = m
    fired = lm.fired_classes(m)
    must_fire, must_not = fx.expand_expect(v["expect"])

    missing_fire = must_fire - fired
    check(not missing_fire,
          f"{name} ({kind}): fires {sorted(must_fire)} — missing {sorted(missing_fire)} "
          f"(fired: {sorted(fired)})")
    false_fire = must_not & fired
    if false_fire:
        detail = ", ".join(
            f"{p['name']}={p['value']:.4f} (floor {p['floor']}, {p['excess']:.1f}x)"
            for p in lm.fired_properties(m) if p["drift_class"] in false_fire)
    else:
        detail = ""
    check(not false_fire,
          f"{name} ({kind}): does NOT fire {sorted(must_not & fired) or '—'} {detail}")

# ═════════════════════════════════════════════════════════════════════════════
# control 1 — the legit set is the universal negative control
# ═════════════════════════════════════════════════════════════════════════════
print("\n── control 1: no legitimate variation fires a layout class ──")
legit = [v["name"] for v in manifest["variants"] if v["kind"] == "legit"]
for name in legit:
    fired = lm.fired_classes(results[name]) & set(lm.LAYOUT_CLASSES)
    check(not fired, f"control 1: legit variant '{name}' fires no layout class, got {sorted(fired)}")

# ═════════════════════════════════════════════════════════════════════════════
# controls 4 and 5 — orthogonality. The two non-negotiable ones.
# ═════════════════════════════════════════════════════════════════════════════
print("\n── controls 4 and 5: orthogonality ──")
dw = results["different_words"]
check(not (lm.fired_classes(dw) & set(lm.LAYOUT_CLASSES)),
      "control 4: different words at identical layout move NO layout property "
      "(a layout metric that moves here is secretly a text metric)")
check(dw["ordered_recall"] < 0.9,
      f"control 4: ...while the content tier craters (ordered_recall {dw['ordered_recall']:.3f})")

sh = results["shift1cm"]
check("margin" in lm.fired_classes(sh), "control 5: a pure geometry change fires geometry")
check("ordering" not in lm.fired_classes(sh),
      f"control 5: ...and does NOT touch the content tier "
      f"(ordered_recall {sh['ordered_recall']:.3f}) — geometry metrics are not secretly text metrics")

# ═════════════════════════════════════════════════════════════════════════════
# control 6 — swap symmetry
# ═════════════════════════════════════════════════════════════════════════════
print("\n── control 6: swap ──")
fwd = fx.metrics_for(base, pdfs["margin"])
rev = fx.metrics_for(pdfs["margin"], base)
check(lm.fired_classes(fwd) & set(lm.LAYOUT_CLASSES) == lm.fired_classes(rev) & set(lm.LAYOUT_CLASSES),
      "control 6: swapping truth and output fires the same layout classes")
check(abs(fwd["layout_text_width_ratio"] * rev["layout_text_width_ratio"] - 1.0) < 1e-6,
      "control 6: the two directions are exact reciprocals")

# ═════════════════════════════════════════════════════════════════════════════
# coverage matrix — every routable class has a real variant behind it
# ═════════════════════════════════════════════════════════════════════════════
print("\n── coverage matrix ──")
covered = set()
for v in manifest["variants"]:
    if v["kind"] == "real":
        covered |= fx.expand_expect(v["expect"])[0]
# `density` and `ink` are consequences of other changes rather than things a
# variant sets directly, and `anchors` needs a LaTeX source rather than a .typ
# pair, so they are covered by layout_metrics_test.py instead.
expected = set(lm.DRIFT_CLASSES) - {"density", "ink", "anchors"}
check(expected <= covered,
      f"every routable drift_class has a REAL variant asserting it fires; "
      f"uncovered: {sorted(expected - covered)}")

# ═════════════════════════════════════════════════════════════════════════════
# known blind spots — asserted, not assumed
# ═════════════════════════════════════════════════════════════════════════════
print("\n── known blind spots ──")
for v in manifest["variants"]:
    if v["kind"] != "known_blind":
        continue
    fired = lm.fired_classes(results[v["name"]]) & set(lm.LAYOUT_CLASSES)
    check(not fired,
          f"known blind spot '{v['name']}' is still invisible to the layout tier "
          f"(if this FAILS the blind spot closed — promote it to kind: real), got {sorted(fired)}")

print("\n── measured profile (for the floors PR) ──")
print(f"{'variant':22s} {'kind':12s} {'severity':>9s}  fired")
for v in manifest["variants"]:
    m = results[v["name"]]
    sev = lm.layout_severity(m)
    print(f"{v['name']:22s} {v['kind']:12s} {sev:9.2f}  {','.join(sorted(lm.fired_classes(m))) or '—'}")

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
