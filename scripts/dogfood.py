#!/usr/bin/env python3
"""
ByeTex dogfood harness — instrument whether the AGENT SURFACE (skills/MCP/CLI) is
self-sufficient for the "last mile" of a conversion.

A FRESH Sonnet agent (the `byetex-dogfood-tester` subagent) repairs ONE seeded
conversion in an isolated sandbox using ONLY the byetex CLI + skills, then emits a
structured friction report. This script does the *deterministic* bookend work —
paper selection, honest sandbox setup, and objective before/after scoring — but it
NEVER runs the agent itself (the orchestrator does that via the Agent tool).

The key reuse insight: neither `byetex review` nor `scripts/visual_test.py` scores
an agent-*edited* `.typ` — both re-convert from `.tex`. So we compile the agent's
edited `main.typ` ourselves and call `visual_test.pdf_structure_compare` on it
directly, with the SAME metric definitions as the corpus fidelity gate.

Usage (run under uv so the SSIM + PDF deps are present, like fidelity_gate.sh):

    uv run --with requests --with Pillow --with numpy --with scikit-image \
        python scripts/dogfood.py select --n 3
    uv run --with requests --with Pillow --with numpy --with scikit-image \
        python scripts/dogfood.py prepare 2605.22765
    uv run --with requests --with Pillow --with numpy --with scikit-image \
        python scripts/dogfood.py score <sandbox> --report report.json

Env: BYETEX_BIN (byetex binary, default `byetex`), BYETEX_TYPST_BIN (default `typst`),
     BYETEX_TECTONIC_BIN (default `tectonic`).
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

# Reuse the corpus fidelity metrics verbatim (one definition of word_recall etc.).
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import visual_test as vt  # noqa: E402  (needs requests + Pillow at import time)

REPO_ROOT = Path(__file__).resolve().parent.parent
SANDBOX_ROOT = REPO_ROOT / "tmp" / "dogfood"
BACKLOG_JSONL = REPO_ROOT / "docs" / "agent-surface-backlog.jsonl"
FIDELITY_BASELINE = REPO_ROOT / "scripts" / "fidelity_baseline.json"
ACCEPTANCE_BASELINE = REPO_ROOT / "scripts" / "acceptance_baseline.json"

# Score-time thresholds for pdf_structure_compare. They only affect the reported
# `structure_ok`/`fail_reasons` fields; the numeric metrics (word_recall etc.) are
# threshold-independent. Kept aligned with the corpus gate's typical bar.
STRUCT_THRESHOLDS = dict(
    page_min=0.70, page_max=1.30, jaccard_min=0.55,
    word_recall_min=0.65, heading_recall_min=0.60,
)
FIDELITY_FLOOR = 0.75  # absolute "good enough" bar; tunable (corpus score ~0.821)
DPI = 100


# ─────────────────────────────────────────────────────────────────────────────
# Small subprocess helpers
# ─────────────────────────────────────────────────────────────────────────────

def _byetex() -> str:
    return os.environ.get("BYETEX_BIN", "byetex")


def _typst() -> str:
    return os.environ.get("BYETEX_TYPST_BIN", "typst")


def compile_typst(typ_path: Path, out_pdf: Path) -> tuple[bool, str, int]:
    """Compile a .typ → PDF (cwd = its dir so relative asset paths resolve).
    Returns (ok, log, error_count). Mirrors visual_test's `--no-pdf-tags` flag
    (typst 0.14 panics on PDF tags for multi-column docs)."""
    out_pdf.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        [_typst(), "compile", "--no-pdf-tags", str(typ_path), str(out_pdf)],
        cwd=str(typ_path.parent), capture_output=True, text=True,
    )
    log = (r.stderr or "") + (r.stdout or "")
    err_count = sum(1 for ln in log.splitlines() if ln.lstrip().startswith("error:"))
    return r.returncode == 0, log, err_count


# ─────────────────────────────────────────────────────────────────────────────
# Objective fidelity scoring of an arbitrary .typ (the reuse seam)
# ─────────────────────────────────────────────────────────────────────────────

def fidelity_of(typ_path: Path, truth_pdf: Path | None, source_tex: str | None,
                work: Path, tag: str) -> dict:
    """Compile `typ_path` and score its PDF against `truth_pdf` with the SAME
    metric functions the corpus gate uses. `work` is a scratch dir for the PDF +
    rasterized pages. Returns a metrics dict incl. `compiled` and `fidelity_score`
    (None when it can't compile or there's no truth to compare against)."""
    work.mkdir(parents=True, exist_ok=True)
    pdf = work / f"{tag}.pdf"
    ok, log, err_count = compile_typst(typ_path, pdf)
    if not ok:
        return {"compiled": False, "fidelity_score": None,
                "typst_error_count": err_count, "compile_log_tail": log[-1500:]}
    if truth_pdf is None or not truth_pdf.exists():
        return {"compiled": True, "fidelity_score": None,
                "typst_error_count": 0, "note": "no truth pdf — compile-only"}

    truth_pngs = vt.rasterize_pdf(truth_pdf, work / "truth", DPI)
    typst_pngs = vt.rasterize_pdf(pdf, work / tag, DPI)
    m = vt.pdf_structure_compare(
        truth_pdf, pdf, len(truth_pngs), len(typst_pngs),
        source_tex=source_tex, typst_tex=typ_path.read_text(encoding="utf-8", errors="replace"),
        **STRUCT_THRESHOLDS,
    )
    ssim = vt.page_image_similarity(truth_pngs, typst_pngs)
    m["mean_ssim"] = ssim.get("mean_ssim")
    m["compiled"] = True
    m["typst_error_count"] = 0
    # Single-paper blend, identical weights to the corpus fidelity_score.
    m["fidelity_score"] = vt.aggregate_fidelity_score({typ_path.stem: m})
    return m


def _slim(m: dict) -> dict:
    """The subset of a fidelity dict worth persisting in the backlog record."""
    keys = ("compiled", "fidelity_score", "word_recall", "heading_recall",
            "page_ratio", "mean_ssim", "structure_ok", "typst_error_count")
    return {k: m.get(k) for k in keys if k in m}


# ─────────────────────────────────────────────────────────────────────────────
# Truth-PDF resolution (cheapest first; offline-friendly)
# ─────────────────────────────────────────────────────────────────────────────

def resolve_truth_pdf(paper_id: str, source_dir: Path, toplevel: Path,
                      dest: Path) -> Path | None:
    """Copy a reference PDF to `dest`. Priority: cached visual_test truth →
    a PDF bundled in source/ → tectonic compile of the toplevel."""
    cached = REPO_ROOT / "tests" / "visual" / paper_id / "truth.pdf"
    if cached.exists():
        shutil.copy2(cached, dest)
        return dest
    bundled = vt.find_existing_truth_pdf(source_dir)
    if bundled is not None:
        shutil.copy2(bundled, dest)
        return dest
    tectonic = os.environ.get("BYETEX_TECTONIC_BIN", "tectonic")
    if shutil.which(tectonic):
        r = subprocess.run(
            [tectonic, "-X", "compile", "--outdir", str(dest.parent), str(toplevel)],
            capture_output=True, text=True,
        )
        produced = dest.parent / (toplevel.stem + ".pdf")
        if r.returncode == 0 and produced.exists():
            if produced != dest:
                shutil.move(str(produced), str(dest))
            return dest
    return None


# ─────────────────────────────────────────────────────────────────────────────
# select
# ─────────────────────────────────────────────────────────────────────────────

def _corpus_ids() -> list[str]:
    return sorted(
        p.name for p in (REPO_ROOT / "corpus").iterdir()
        if p.is_dir() and p.name != "_out" and (p / "source").exists()
    )


def cmd_select(args) -> int:
    """Rank candidate papers hardest-first: BYETEX_FAIL papers, then lowest
    word_recall. Deterministic so re-runs target the same set."""
    fid = json.loads(FIDELITY_BASELINE.read_text()).get("papers", {}) \
        if FIDELITY_BASELINE.exists() else {}
    acc = json.loads(ACCEPTANCE_BASELINE.read_text()) if ACCEPTANCE_BASELINE.exists() else {}
    known_fail = set(acc.get("known_fail", []))

    ranked = []
    for pid in _corpus_ids():
        rec = fid.get(pid, {})
        failing = pid in known_fail or rec.get("structure_ok") is False \
            or (rec.get("status") not in (None, "ok"))
        wr = rec.get("word_recall")
        measured = wr is not None
        # tier 0 = failing; then ascending word_recall; unmeasured treated as 0.5.
        key = (0 if failing else 1, wr if measured else 0.5)
        reason = ("BYETEX_FAIL/structure-fail" if failing
                  else f"word_recall={wr}" if measured else "unmeasured")
        ranked.append((key, pid, reason, wr))
    ranked.sort(key=lambda t: t[0])
    picked = ranked[: args.n]

    if args.json:
        print(json.dumps([{"id": p, "reason": r, "word_recall": w}
                          for _, p, r, w in picked]))
    else:
        for _, pid, reason, _w in picked:
            print(f"{pid}\t{reason}")
    return 0


# ─────────────────────────────────────────────────────────────────────────────
# prepare
# ─────────────────────────────────────────────────────────────────────────────

def cmd_prepare(args) -> int:
    paper_id = args.paper_id
    source_dir = vt.find_source_dir(paper_id)
    if source_dir is None:
        print(f"error: no corpus source for {paper_id}", file=sys.stderr)
        return 2
    toplevel = vt.find_toplevel_tex(source_dir)
    if toplevel is None:
        print(f"error: cannot resolve toplevel .tex for {paper_id}", file=sys.stderr)
        return 2

    ts = args.run_ts or datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    sandbox = SANDBOX_ROOT / paper_id / ts
    stage = SANDBOX_ROOT / paper_id / f".stage-{ts}"   # source lives OUTSIDE the sandbox
    priv = sandbox / ".dogfood"                          #  while diagnose materializes it
    src_copy = sandbox / "src"
    for d in (sandbox, stage):
        if d.exists():
            shutil.rmtree(d)
    stage.mkdir(parents=True)
    sandbox.parent.mkdir(parents=True, exist_ok=True)

    # 1. Stage the pristine LaTeX project (skip the tarball; keep .bbl/.bst/figs).
    for item in source_dir.iterdir():
        if item.name == "source.tar.gz":
            continue
        if item.is_dir():
            shutil.copytree(item, stage / item.name)
        else:
            shutil.copy2(item, stage / item.name)
    stage_top = stage / toplevel.name

    # 2. Materialize the seed typst project + diagnostics into the CLEAN sandbox.
    #    `diagnose --project --out <dir>` wipes <dir> first, so source must be staged
    #    elsewhere; it writes main.typ even on compile errors plus main.diagnostics.json
    #    (errors mapped → src fragment + repair skill).
    diag = subprocess.run(
        [_byetex(), "diagnose", str(stage_top), "--project", "--out", str(sandbox)],
        capture_output=True, text=True,
    )
    main_typ = sandbox / "main.typ"
    if not main_typ.exists():
        print(f"error: diagnose did not produce {main_typ}\n{diag.stderr}", file=sys.stderr)
        return 2
    # warnings.json: prefer the materialized one; fall back to the corpus _out seed.
    if not (sandbox / "warnings.json").exists() and not (sandbox / "main.warnings.json").exists():
        out_warn = REPO_ROOT / "corpus" / "_out" / paper_id / "warnings.json"
        if out_warn.exists():
            shutil.copy2(out_warn, sandbox / "warnings.json")

    # 3. Now that diagnose has run, it's safe to add the private dir + the source copy
    #    (the agent gets a realistic src/ but is told diagnostics are pre-computed —
    #    re-running `diagnose --out .` would wipe its edits).
    priv.mkdir(parents=True)
    shutil.move(str(stage), str(src_copy))
    sb_toplevel = src_copy / toplevel.name
    shutil.copy2(main_typ, priv / "seed.typ")   # immutable backup (text diff / recovery)

    # 4. Truth PDF + pre-rasterized truth pages for the agent's visual comparison.
    truth = resolve_truth_pdf(paper_id, source_dir, sb_toplevel, sandbox / "truth.pdf")
    if truth is not None:
        try:
            vt.rasterize_pdf(truth, sandbox / "truth-pages" / "truth", DPI)
        except Exception:
            pass

    # 5. fidelity_before from the seed — compile main.typ IN PLACE (cwd=sandbox) so its
    #    relative asset paths (figures/, *.bib) resolve.
    source_tex = vt.collect_project_source(sb_toplevel)
    before = fidelity_of(main_typ, truth, source_tex, priv / "before", "seed")

    (priv / "before.json").write_text(json.dumps(before, indent=2))
    (priv / "meta.json").write_text(json.dumps({
        "paper_id": paper_id, "run_ts": ts,
        "toplevel": str(sb_toplevel.relative_to(sandbox)),
        "truth_pdf": "truth.pdf" if truth else None,
        "diagnostics": "main.diagnostics.json" if (sandbox / "main.diagnostics.json").exists() else None,
    }, indent=2))

    # Human-readable banner to stderr; the LAST stdout line is the bare path (for capture).
    n_diag = 0
    dj = sandbox / "main.diagnostics.json"
    if dj.exists():
        try:
            n_diag = len(json.loads(dj.read_text()))
        except Exception:
            pass
    print(f"[prepare] {paper_id} @ {ts}", file=sys.stderr)
    print(f"  seed compiled={before.get('compiled')} "
          f"fidelity_before={before.get('fidelity_score')} "
          f"diagnostics={n_diag} truth={'yes' if truth else 'NO'}", file=sys.stderr)
    print(str(sandbox))
    return 0


# ─────────────────────────────────────────────────────────────────────────────
# score
# ─────────────────────────────────────────────────────────────────────────────

def _verdict(report: dict, after: dict, before: dict, mismatch: bool) -> str:
    if not after.get("compiled") or mismatch:
        return "NEEDS_FIX"
    for sp in report.get("stuck_points", []) or []:
        if sp.get("resolution") in ("workaround", "gave_up"):
            return "NEEDS_FIX"
    for u in report.get("unclear_skill_notes", []) or []:
        if u.get("severity") in ("blocker", "major"):
            return "NEEDS_FIX"
    fa = after.get("fidelity_score")
    fb = before.get("fidelity_score")
    if fa is None:                       # couldn't measure fidelity → conservative
        return "NEEDS_FIX"
    if fa < FIDELITY_FLOOR:
        return "NEEDS_FIX"
    if fb is not None and fa < fb - 1e-3:  # regressed vs the seed
        return "NEEDS_FIX"
    return "GOOD_ENOUGH"


def cmd_score(args) -> int:
    sandbox = Path(args.sandbox).resolve()
    priv = sandbox / ".dogfood"
    meta = json.loads((priv / "meta.json").read_text())
    before = json.loads((priv / "before.json").read_text())

    raw = sys.stdin.read() if args.report == "-" else Path(args.report).read_text()
    report = _extract_report(raw)

    truth = sandbox / meta["truth_pdf"] if meta.get("truth_pdf") else None
    toplevel = sandbox / meta["toplevel"]
    source_tex = vt.collect_project_source(toplevel) if toplevel.exists() else None
    after = fidelity_of(sandbox / "main.typ", truth, source_tex, priv / "after", "after")

    mismatch = bool(report.get("compiled")) != bool(after.get("compiled"))
    verdict = _verdict(report, after, before, mismatch)

    fb, fa = before.get("fidelity_score"), after.get("fidelity_score")
    delta = round(fa - fb, 3) if (fa is not None and fb is not None) else None

    record = {
        "run_ts": meta["run_ts"], "paper_id": meta["paper_id"],
        "verdict": verdict, "self_report_mismatch": mismatch,
        "compiled": after.get("compiled"),
        "iterations": report.get("iterations"),
        "fidelity_before": _slim(before), "fidelity_after": _slim(after),
        "delta_fidelity": delta,
        "stuck_points": report.get("stuck_points", []),
        "missing_tool_wishlist": report.get("missing_tool_wishlist", []),
        "unclear_skill_notes": report.get("unclear_skill_notes", []),
        "final_typst_errors": report.get("final_typst_errors", []),
        "notes": report.get("notes"),
    }
    BACKLOG_JSONL.parent.mkdir(parents=True, exist_ok=True)
    with BACKLOG_JSONL.open("a") as fh:
        fh.write(json.dumps(record) + "\n")

    print(f"[score] {meta['paper_id']}  verdict={verdict}", file=sys.stderr)
    print(f"  fidelity_before={fb}  fidelity_after={fa}  delta={delta}", file=sys.stderr)
    if mismatch:
        print("  ⚠ self_report_mismatch: agent's compiled flag != actual", file=sys.stderr)
    n_friction = (len(record["stuck_points"]) + len(record["missing_tool_wishlist"])
                  + len(record["unclear_skill_notes"]))
    print(f"  friction items logged: {n_friction} → {BACKLOG_JSONL.relative_to(REPO_ROOT)}",
          file=sys.stderr)
    print(verdict)
    return 0


def _extract_report(raw: str) -> dict:
    """Parse the agent's dogfood report. Accepts a bare JSON object or text with a
    trailing ```json fenced block (the agent emits the latter)."""
    raw = raw.strip()
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        pass
    if "```" in raw:
        # take the LAST fenced block
        blocks = raw.split("```")
        for chunk in reversed(blocks):
            c = chunk.strip()
            if c.startswith("json"):
                c = c[4:].strip()
            if c.startswith("{"):
                try:
                    return json.loads(c)
                except json.JSONDecodeError:
                    continue
    # last resort: slice from the first { to the last }
    i, j = raw.find("{"), raw.rfind("}")
    if i != -1 and j != -1 and j > i:
        try:
            return json.loads(raw[i:j + 1])
        except json.JSONDecodeError:
            pass
    print("warning: could not parse a dogfood report; treating as empty",
          file=sys.stderr)
    return {}


# ─────────────────────────────────────────────────────────────────────────────

def main() -> int:
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)

    ps = sub.add_parser("select", help="rank hardest candidate papers")
    ps.add_argument("--n", type=int, default=3)
    ps.add_argument("--json", action="store_true")
    ps.set_defaults(func=cmd_select)

    pp = sub.add_parser("prepare", help="build an honest sandbox + score the seed")
    pp.add_argument("paper_id")
    pp.add_argument("--run-ts", default=None, help="override the timestamp dir name")
    pp.set_defaults(func=cmd_prepare)

    pc = sub.add_parser("score", help="score the agent-edited main.typ + log friction")
    pc.add_argument("sandbox")
    pc.add_argument("--report", default="-", help="report JSON path, or - for stdin")
    pc.set_defaults(func=cmd_score)

    args = p.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
