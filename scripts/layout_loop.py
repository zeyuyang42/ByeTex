#!/usr/bin/env python3
"""Loop driver for the layout tier. Currently: `floors`.

  floors   measure the NOISE FLOORS a layout property must clear before it can
           be promoted to a hard gate, and (optionally) emit them as JSON.

Why floors are measured rather than chosen
──────────────────────────────────────────
The one rule that governs this file:

    CORPUS PERCENTILES MUST NEVER BECOME THRESHOLDS.

50 of 65 corpus papers already drift on leading. A corpus percentile therefore
measures ByeTex's *systematic bias* and would encode the bug as normal — the
gate would pass precisely because everything is broken. The corpus distribution
is a WORKLOAD DESCRIPTION, not a tolerance, and this script prints the two side
by side so the gap between them stays visible.

A tolerance answers "is this difference beyond what the INSTRUMENT and
LEGITIMATE ENGINE DIFFERENCES can produce?" Three measured sources answer that:

  self_floor    each truth.pdf profiled against ITSELF. Every ratio must be
                EXACTLY 1.0 on every paper. If it is not, extraction is
                nondeterministic and every number downstream is void. This is a
                precondition, not a tolerance — it gates the whole exercise.

  repeat_floor  the same .typ compiled twice, profiled both times. The
                instrument's own noise: typst's own determinism plus ours.

  legit_floor   the largest per-property deviation across the synthetic LEGIT
                variants — differences a converter must be FORGIVEN for (a
                different font family, ragged vs justified, half-point size
                drift, hyphenation policy). This is the real lower bound: a
                threshold under it fires on correct output.

A property is promotable only when all four hold:
  (a) no synthetic legit variant trips the proposed threshold;
  (b) repeat_floor is ~0;
  (c) at most ~10 corpus papers fail it — a backlog, not a wall;
  (d) each of those is MANUALLY CONFIRMED as real drift.
(d) is the step everyone skips and the entire difference between a gate and
noise. This script can establish (a), (b) and (c). It cannot establish (d), and
says so rather than implying a number is ready.

Usage:
  uv run --with pymupdf python scripts/layout_loop.py floors \\
      --index tests/visual/index.json --synthetic --repeat 8 \\
      --emit scripts/layout_floors.json
"""
from __future__ import annotations

import argparse
import json
import statistics as st
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import layout_metrics as lm  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parent.parent

# Properties floors are measured for. `layout_column_mismatch_frac` is absent on
# purpose: its detector still disagrees on ~13% of pages, so a floor measured for
# it would be a floor for the detector's noise, not the converter's drift.
FLOOR_PROPERTIES = tuple(
    p for props in lm.DRIFT_CLASSES.values() for p in props
    if p.startswith("layout_") and p != "layout_column_mismatch_frac"
)

# Where a drift_class most likely comes from. This table is what makes a loop
# tick ACTIONABLE: without it the backlog is a column of numbers, and "leading is
# 21% off on 50 papers" tells nobody which file to open. It is the deterministic
# analogue of the dogfood routing rubric — a starting point for the search, not a
# diagnosis, which is why each entry names a reason rather than just a path.
DRIFT_CLASS_ROUTES = {
    "page_trim": ("emit/preamble.rs, style_profile.rs",
                  "page size comes from the class options and the neutral preamble"),
    "margin": ("style_profile.rs, emit/preamble.rs",
               "margins are set per DocClass in the generated `#set page(...)`"),
    "text_width": ("style_profile.rs, emit/preamble.rs",
                   "text measure follows from page size minus margins; check both"),
    "font_size": ("class_map.rs, style_profile.rs",
                  "the `10pt`/`11pt`/`12pt` class option resolves to the body size"),
    "type_scale": ("style_profile.rs, emit/typography.rs",
                   "heading and body sizes are emitted as a scale, not independently"),
    "small_tier": ("emit/typography.rs",
                   "\\small / \\footnotesize / \\scriptsize size switches"),
    "leading": ("emit/preamble.rs",
                "`#set par(leading:)` in the neutral preamble vs the class's baselineskip"),
    "density": ("emit/preamble.rs, style_profile.rs",
                "lines per page follows leading and margins; fix those first"),
    "columns": ("emit/preamble.rs, style_profile.rs",
                "`#set page(columns: 2)` and the spanning-title float"),
    "ink": ("emit/figures.rs, emit/tables.rs",
            "ink density is dominated by floats and tables being dropped or resized"),
    "anchors": ("ir.rs, emit/sections.rs",
                "\\label keys are normalised pre-parse and emitted as `<key>` anchors"),
    "ordering": ("emit.rs, emit/macros.rs",
                 "dropped or duplicated content — check \\input handling and unsupported commands"),
}

SAFETY_FACTOR = 1.5  # applied to max(repeat_floor, legit_floor)

# Floor of last resort. Several properties have a legit_floor of EXACTLY zero —
# no synthetic legit variant moves them at all (a font swap does not change the
# page size). Taken literally that yields a threshold of 0.0, which fires on any
# deviation whatsoever including PDF coordinate rounding.
#
# This is the one number here NOT derived from measurement, so it is stated
# plainly rather than buried: 0.5% of a ratio is below what a reader can perceive
# and within the rounding of the coordinates being compared. It is a resolution
# limit, not a tolerance — and where it binds, the emitted JSON records
# `threshold_source: "resolution_floor"` so nobody mistakes it for evidence.
MIN_THRESHOLD = 0.005


def _tool_version(cmd: list[str]) -> str:
    try:
        return subprocess.run(cmd, capture_output=True, text=True, check=True).stdout.strip()
    except Exception:  # noqa: BLE001
        return "unknown"


def _pymupdf_version() -> str:
    try:
        import fitz  # noqa: PLC0415
        return f"pymupdf {getattr(fitz, '__doc__', '').strip() or 'unknown'}"
    except ImportError:
        return "absent"


def measure_self_floor(paper_dirs: list[Path]) -> dict:
    """Profile each truth.pdf against ITSELF.

    The precondition for everything else: identical input must give identical
    output. A non-1.0 here means the extraction path is nondeterministic and no
    downstream comparison means anything.
    """
    worst: dict[str, float] = {}
    offenders: list[str] = []
    n = 0
    for d in paper_dirs:
        truth = d / "truth.pdf"
        if not truth.exists():
            continue
        prof = lm.profile_pdf(truth)
        if prof is None:
            continue
        res = lm.compare_profiles(prof, prof)
        n += 1
        bad = False
        for name in FLOOR_PROPERTIES:
            v = res.get(name)
            if v is None:
                continue
            dev = lm.property_deviation(name, v)
            worst[name] = max(worst.get(name, 0.0), dev)
            if dev > 0.0:
                bad = True
        if bad:
            offenders.append(d.name)
    return {"n": n, "worst": worst, "offenders": offenders}


def measure_repeat_floor(paper_dirs: list[Path], limit: int) -> dict:
    """Compile each paper's generated main.typ a SECOND time and compare.

    Isolates the instrument's own noise from the converter's. Uses the already
    generated project so no conversion is re-run — only typst.
    """
    worst: dict[str, float] = {}
    offenders: list[str] = []
    n = 0
    tmp = REPO_ROOT / "target" / "layout-floors"
    tmp.mkdir(parents=True, exist_ok=True)
    for d in paper_dirs:
        if n >= limit:
            break
        pid = d.name
        proj = REPO_ROOT / "corpus" / "_out" / pid / "main.typ"
        first = d / "typst.pdf"
        if not (proj.exists() and first.exists()):
            continue
        again = tmp / f"{pid}.pdf"
        r = subprocess.run(
            ["typst", "compile", "--no-pdf-tags", str(proj), str(again)],
            capture_output=True, text=True, cwd=proj.parent,
        )
        if r.returncode != 0:
            continue
        a, b = lm.profile_pdf(first), lm.profile_pdf(again)
        if a is None or b is None:
            continue
        res = lm.compare_profiles(a, b)
        n += 1
        bad = False
        for name in FLOOR_PROPERTIES:
            v = res.get(name)
            if v is None:
                continue
            dev = lm.property_deviation(name, v)
            worst[name] = max(worst.get(name, 0.0), dev)
            if dev > 1e-9:
                bad = True
        if bad:
            offenders.append(pid)
    return {"n": n, "worst": worst, "offenders": offenders}


def measure_legit_floor() -> dict:
    """Largest per-property deviation across the synthetic LEGIT variants.

    These are differences a converter must be FORGIVEN for. A threshold below
    this floor fires on correct output, which is the fastest way to get a gate
    switched off.
    """
    sys.path.insert(0, str(Path(__file__).resolve().parent / "tests"))
    import _layout_fixtures as fx  # noqa: PLC0415

    missing = fx.missing_tools()
    if missing:
        return {"n": 0, "worst": {}, "skipped": f"missing {', '.join(missing)}"}
    manifest = fx.load_manifest()
    pdfs = fx.build_all(manifest)
    base = pdfs["base"]
    worst: dict[str, float] = {}
    per_variant: dict[str, dict] = {}
    n = 0
    for v in manifest["variants"]:
        # `different_words` is legit for LAYOUT only — its whole job is to change
        # content — so it belongs to the orthogonality control, not this floor.
        if v["kind"] != "legit" or v["expect"].get("must_fire"):
            continue
        res = lm.layout_compare(base, pdfs[v["name"]])
        n += 1
        pv = {}
        for name in FLOOR_PROPERTIES:
            val = res.get(name)
            if val is None:
                continue
            dev = lm.property_deviation(name, val)
            worst[name] = max(worst.get(name, 0.0), dev)
            pv[name] = round(dev, 4)
        per_variant[v["name"]] = pv
    return {"n": n, "worst": worst, "per_variant": per_variant}


def corpus_distribution(index_path: Path) -> dict:
    """The WORKLOAD description — never a tolerance. Reported so the distance
    between the floor and where the corpus actually sits stays visible."""
    papers = json.loads(index_path.read_text()).get("papers", {})
    out: dict[str, dict] = {}
    for name in FLOOR_PROPERTIES:
        devs = [lm.property_deviation(name, p[name])
                for p in papers.values() if p.get(name) is not None]
        undefined = sum(1 for p in papers.values() if p.get(name) is None)
        if not devs:
            out[name] = {"n": 0, "n_undefined": undefined}
            continue
        devs.sort()
        med = st.median(devs)
        out[name] = {
            "n": len(devs),
            "n_undefined": undefined,
            "p05": round(devs[int(0.05 * (len(devs) - 1))], 4),
            "p50": round(med, 4),
            "p95": round(devs[int(0.95 * (len(devs) - 1))], 4),
            "mad": round(st.median([abs(d - med) for d in devs]), 4),
        }
    return out


def cmd_floors(args) -> int:
    index_path = Path(args.index) if args.index else REPO_ROOT / "tests" / "visual" / "index.json"
    paper_dirs = sorted(d for d in index_path.parent.iterdir()
                        if d.is_dir() and (d / "truth.pdf").exists())

    print("── self_floor (each truth.pdf vs ITSELF — must be EXACTLY 1.0) ──")
    self_f = measure_self_floor(paper_dirs)
    self_ok = not self_f["offenders"]
    print(f"   {self_f['n']} papers profiled; "
          + ("all exactly 1.0 ✓" if self_ok
             else f"NONDETERMINISTIC on {len(self_f['offenders'])}: {self_f['offenders'][:6]}"))
    if not self_ok:
        print("\n   STOP. Extraction is not deterministic, so every number below is\n"
              "   meaningless. Fix extraction before measuring any tolerance.", file=sys.stderr)
        return 1

    print(f"\n── repeat_floor (same .typ compiled twice, up to {args.repeat} papers) ──")
    rep = measure_repeat_floor(paper_dirs, args.repeat)
    # n == 0 must NOT read as a pass. An empty measurement printing "identical on
    # every property ✓" is exactly the vacuous control this project has been bitten
    # by twice; it happened here the moment corpus/_out was not reachable.
    if rep["n"] == 0:
        print("   NOT MEASURED — no generated corpus/_out/<id>/main.typ found. "
              "repeat_floor is UNKNOWN, not zero.", file=sys.stderr)
        rep["worst"] = {}
        rep["unmeasured"] = True
    else:
        print(f"   {rep['n']} papers recompiled; "
              + ("identical on every property ✓" if not rep["offenders"]
                 else f"differs on {rep['offenders']}"))

    legit = {"n": 0, "worst": {}}
    if args.synthetic:
        print("\n── legit_floor (synthetic variants a converter must be FORGIVEN for) ──")
        legit = measure_legit_floor()
        if legit.get("skipped"):
            print(f"   SKIPPED — {legit['skipped']}")
        else:
            print(f"   {legit['n']} legit variants measured")

    corpus = corpus_distribution(index_path)

    print("\n── proposed thresholds ──")
    print("   threshold = max(repeat_floor, legit_floor) x %.1f\n" % SAFETY_FACTOR)
    print(f"{'property':34s} {'repeat':>8s} {'legit':>8s} {'PROPOSED':>9s} "
          f"{'corpus p50':>11s} {'fail@thr':>9s}")
    proposals = {}
    papers = json.loads(index_path.read_text()).get("papers", {})
    for name in FLOOR_PROPERTIES:
        r = rep["worst"].get(name, 0.0)
        lg = legit["worst"].get(name)
        if lg is None:
            print(f"{name:34s} {r:8.4f} {'—':>8s} {'—':>9s} "
                  f"{corpus[name].get('p50', float('nan')):11.4f} {'—':>9s}")
            continue
        measured = max(r, lg) * SAFETY_FACTOR
        thr = max(measured, MIN_THRESHOLD)
        source = "measured" if measured >= MIN_THRESHOLD else "resolution_floor"
        fails = sum(1 for p in papers.values() if p.get(name) is not None
                    and lm.property_deviation(name, p[name]) > thr)
        proposals[name] = {"repeat_floor": round(r, 5), "legit_floor": round(lg, 5),
                           "proposed_threshold": round(thr, 5),
                           "threshold_source": source,
                           "corpus_fail_at_threshold": fails,
                           "promotable": fails <= 10}
        mark = "  <- promotable" if fails <= 10 else ""
        star = "*" if source == "resolution_floor" else " "
        print(f"{name:34s} {r:8.4f} {lg:8.4f} {thr:8.4f}{star} "
              f"{corpus[name].get('p50', 0):11.4f} {fails:9d}{mark}")

    print(f"\n   * threshold is the {MIN_THRESHOLD} resolution floor, not measured "
          "evidence:\n     no legit variant moves that property at all.")
    print("\n   `corpus p50` is a WORKLOAD DESCRIPTION, not a candidate threshold.")
    print("   A property is promotable only when the proposed threshold is clear of")
    print("   both floors AND at most ~10 papers fail it AND each of those has been")
    print("   MANUALLY CONFIRMED as real drift. This tool cannot do the last step.")

    if args.emit:
        payload = {
            "_comment": (
                "Measured noise floors for the layout tier (scripts/layout_loop.py "
                "floors). FROZEN: change only by explicit PR, never as a side effect "
                "of a run. An auto-updating floor tracks the regression and is worse "
                "than no floor. `corpus_*` fields describe the WORKLOAD and must "
                "never be adopted as thresholds."
            ),
            "typst_version": _tool_version(["typst", "--version"]),
            "pymupdf_version": _pymupdf_version(),
            "safety_factor": SAFETY_FACTOR,
            "self_floor_ok": self_ok,
            "self_floor_papers": self_f["n"],
            "repeat_floor_papers": rep["n"],
            "repeat_floor_measured": not rep.get("unmeasured", False),
            "legit_variants": legit.get("n", 0),
            "legit_per_variant": legit.get("per_variant", {}),
            "properties": {
                name: {**proposals.get(name, {}), "corpus": corpus.get(name, {})}
                for name in FLOOR_PROPERTIES
            },
        }
        Path(args.emit).write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
        print(f"\nwrote {args.emit}")
    return 0


# ═══════════════════════════════════════════════════════════════════════════
# rank / record / triage — the loop surface
# ═══════════════════════════════════════════════════════════════════════════

def rank_layout_offenders(papers: dict, floors: dict | None = None) -> list[dict]:
    """Papers ordered by how far past their floors they sit. PURE.

    One deliberate departure from `dogfood.rank_candidates`: a paper whose tier
    DID NOT RUN is emitted with `severity: None` and `reason: "unmeasured"`
    rather than dropped. Dropping them would hide exactly the papers most likely
    to be badly laid out — the books and theses whose truth render fails carry
    the corpus's worst geometry, and a ranking that silently omits them reports a
    healthier corpus than exists.
    """
    tols = {}
    for name, info in (floors or {}).get("properties", {}).items():
        thr = info.get("proposed_threshold")
        if thr is not None:
            tols[name] = thr

    prop_class = {p: cls for cls, props in lm.DRIFT_CLASSES.items() for p in props}
    out = []
    for pid, v in sorted(papers.items()):
        fired = []
        measured = False
        for name, thr in sorted(tols.items()):
            val = v.get(name)
            if val is None:
                continue
            measured = True
            dev = lm.property_deviation(name, val)
            if thr and dev > thr:
                fired.append({"name": name, "value": val, "deviation": dev,
                              "threshold": thr, "excess": dev / thr,
                              "drift_class": prop_class.get(name, "unknown")})
        if not measured:
            out.append({"paper_id": pid, "severity": None, "worst_property": None,
                        "drift_class": None, "reason": "unmeasured",
                        "properties_out_of_floor": []})
            continue
        fired.sort(key=lambda d: -d["excess"])
        sev = fired[0]["excess"] if fired else 0.0
        out.append({
            "paper_id": pid,
            "severity": round(sev, 3),
            "worst_property": fired[0]["name"] if fired else None,
            "drift_class": fired[0]["drift_class"] if fired else None,
            "reason": (f"{len(fired)} propert{'y' if len(fired) == 1 else 'ies'} "
                       f"beyond floor" if fired else "within floor"),
            "properties_out_of_floor": fired,
        })
    # Unmeasured papers sort LAST but are never dropped; measured papers by
    # severity descending.
    return sorted(out, key=lambda d: (d["severity"] is None, -(d["severity"] or 0)))


def _load_floors(path: str | None) -> dict:
    fp = Path(path) if path else REPO_ROOT / "scripts" / "layout_floors.json"
    if not fp.is_absolute():
        fp = REPO_ROOT / fp
    return json.loads(fp.read_text()) if fp.exists() else {}


def cmd_rank(args) -> int:
    index = Path(args.index) if args.index else REPO_ROOT / "tests" / "visual" / "index.json"
    papers = json.loads(index.read_text()).get("papers", {})
    rows = rank_layout_offenders(papers, _load_floors(args.floors))
    if args.n:
        rows = rows[:args.n]
    if args.json:
        print(json.dumps(rows, indent=2))
        return 0
    for r in rows:
        sev = "—" if r["severity"] is None else f"{r['severity']:.2f}"
        print(f"{r['paper_id']}\t{sev}\t{r['worst_property'] or '—'}\t{r['reason']}")
    return 0


def cmd_record(args) -> int:
    """Recompute the tier for ONE paper from artifacts already on disk.

    A fast recheck after an emitter fix, without a corpus run.
    """
    out_dir = Path(args.out) if args.out else REPO_ROOT / "tests" / "visual" / args.paper_id
    truth, typ = out_dir / "truth.pdf", out_dir / "typst.pdf"
    rec = {"paper_id": args.paper_id, "metrics": {}}
    if truth.exists() and typ.exists():
        rec["metrics"] = lm.layout_compare(truth, typ)
    else:
        rec["skipped"] = f"need both {truth.name} and {typ.name} in {out_dir}"
    floors = _load_floors(args.floors)
    ranked = rank_layout_offenders({args.paper_id: rec["metrics"]}, floors)[0]
    rec.update({k: ranked[k] for k in
                ("severity", "worst_property", "drift_class", "reason",
                 "properties_out_of_floor")})
    if rec.get("drift_class") in DRIFT_CLASS_ROUTES:
        site, why = DRIFT_CLASS_ROUTES[rec["drift_class"]]
        rec["suspected_site"] = site
        rec["routing_reason"] = why
    dest = out_dir / "layout.json"
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(rec, indent=2, sort_keys=True) + "\n")
    print(dest)  # bare path as the last line — mirrors `dogfood.py prepare`
    return 0


BACKLOG = REPO_ROOT / "docs" / "layout-backlog.jsonl"


def cmd_triage(args) -> int:
    rec = json.loads(Path(args.record).read_text())
    verdict = "DRIFT" if (rec.get("properties_out_of_floor") or []) else "WITHIN_FLOOR"
    floors = _load_floors(args.floors)
    entry = {
        "run_ts": args.run_ts or "unset",
        "schema": 1,
        "paper_id": rec.get("paper_id"),
        "verdict": verdict,
        "source": "layout_loop.triage",
        "severity": rec.get("severity"),
        "worst_property": rec.get("worst_property"),
        "drift_class": rec.get("drift_class"),
        "suspected_site": rec.get("suspected_site"),
        "routing_reason": rec.get("routing_reason"),
        "properties_out_of_floor": rec.get("properties_out_of_floor", []),
        "typst_version": floors.get("typst_version"),
        "pymupdf_version": floors.get("pymupdf_version"),
        "note": args.note,
    }
    BACKLOG.parent.mkdir(parents=True, exist_ok=True)
    with BACKLOG.open("a") as fh:
        fh.write(json.dumps(entry, sort_keys=True) + "\n")
    print(verdict)  # bare verdict word — mirrors `dogfood.py score`
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    f = sub.add_parser("floors", help="measure the layout tier's noise floors")
    f.add_argument("--index", help="index.json (default tests/visual/index.json)")
    f.add_argument("--repeat", type=int, default=8, metavar="N",
                   help="papers to recompile for repeat_floor (default 8)")
    f.add_argument("--synthetic", action="store_true",
                   help="also measure legit_floor from the synthetic fixtures")
    f.add_argument("--emit", metavar="PATH", help="write the measured floors as JSON")
    f.set_defaults(func=cmd_floors)

    r = sub.add_parser("rank", help="papers ordered by how far past their floors they sit")
    r.add_argument("--n", type=int, default=0, metavar="N", help="show only the top N")
    r.add_argument("--json", action="store_true", help="emit a JSON array")
    r.add_argument("--index")
    r.add_argument("--floors")
    r.set_defaults(func=cmd_rank)

    rc = sub.add_parser("record", help="recompute the tier for ONE paper from disk")
    rc.add_argument("paper_id")
    rc.add_argument("--index")
    rc.add_argument("--out", help="paper artifact dir (default tests/visual/<id>)")
    rc.add_argument("--floors")
    rc.set_defaults(func=cmd_record)

    t = sub.add_parser("triage", help="append a routed verdict to docs/layout-backlog.jsonl")
    t.add_argument("paper_id")
    t.add_argument("--record", required=True, help="layout.json written by `record`")
    t.add_argument("--note")
    t.add_argument("--floors")
    t.add_argument("--run-ts", help="timestamp to stamp on the record")
    t.set_defaults(func=cmd_triage)

    args = ap.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
