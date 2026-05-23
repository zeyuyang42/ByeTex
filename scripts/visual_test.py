#!/usr/bin/env python3
"""
ByeTex visual regression test — produces side-by-side composites for agent grading.

For each arXiv paper:
  1. Reads entry .tex from source/00README.json
  2. Downloads the arXiv canonical PDF (or uses the one bundled in the source dir)
  3. Runs bytetex convert → .typ + .warnings.json
  4. Compiles with typst → typst.pdf
  5. Rasterizes both PDFs with pdftoppm
  6. Stacks pages into a side-by-side composite.png for agent visual grading

Usage:
    uv run --with requests --with Pillow python scripts/visual_test.py
    uv run --with requests --with Pillow python scripts/visual_test.py --papers 2605.22507
    uv run --with requests --with Pillow python scripts/visual_test.py --skip-existing
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
import time
import random
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

import requests
from PIL import Image, ImageDraw

# ─────────────────────────────────────────────────────────────────────────────
# Paths & defaults
# ─────────────────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).parent.parent.resolve()
CORPUS_ARXIV = REPO_ROOT / "corpus" / "online" / "arxiv"

ARXIV_PDF_URL = "https://arxiv.org/pdf/{id}"
ARXIV_MIN_DELAY = 3.0
DEFAULT_UA = (
    "ByeTex-Harvester/0.1 (+https://github.com/zeyuyang42/ByeTex; "
    "research/testing use only)"
)

# 5 hand-picked papers: diverse shapes, 3 already have canonical PDFs on disk
DEFAULT_PAPERS = [
    "2605.22507",  # stat.ML — multi-file \input, math-heavy; has 0-main.pdf
    "2605.22557",  # math.NA — math-heavy; has main_sinum.pdf
    "2605.22776",  # cs.LG  — single-file main_en.tex; needs arXiv PDF
    "2605.22159",  # math.NA — multi-file + custom \newcommands; needs arXiv PDF
    "2605.22820",  # cs.LG  — exercises the PDF download path
]

COMPOSITE_CELL_W = 600  # px per column in composite image
MAX_COMPOSITE_PAGES = 12  # truncate after this many rows to keep file sizes down
RASTERIZE_DPI = 100


# ─────────────────────────────────────────────────────────────────────────────
# HTTP helpers
# ─────────────────────────────────────────────────────────────────────────────

def make_session(ua: str) -> requests.Session:
    s = requests.Session()
    s.headers["User-Agent"] = ua
    return s


def fetch(session: requests.Session, url: str, stream: bool = False, **kwargs) -> requests.Response:
    last_err: Exception | None = None
    for attempt in range(3):
        try:
            r = session.get(url, stream=stream, timeout=60, **kwargs)
            if r.status_code < 500:
                return r
            wait = 2 ** attempt * 2
            print(f"  HTTP {r.status_code} for {url!r}, retry in {wait}s", file=sys.stderr)
            time.sleep(wait)
        except (requests.exceptions.Timeout, requests.exceptions.ConnectionError) as exc:
            last_err = exc
            wait = 2 ** attempt * 2
            print(f"  {exc!r}, retry in {wait}s", file=sys.stderr)
            time.sleep(wait)
    raise RuntimeError(f"Exhausted retries for {url}: {last_err}")


def sleep_politely(base: float) -> None:
    time.sleep(base + random.uniform(0, 0.5))


def _now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


# ─────────────────────────────────────────────────────────────────────────────
# arXiv source layout
# ─────────────────────────────────────────────────────────────────────────────

def find_source_dir(arxiv_id: str) -> Path | None:
    """Find the source/ directory for an arXiv ID under corpus/online/arxiv/."""
    id_safe = arxiv_id.replace("/", "_")
    candidate = CORPUS_ARXIV / id_safe / "source"
    return candidate if candidate.exists() else None


def find_toplevel_tex(source_dir: Path) -> Path | None:
    """Return the entry-point .tex by reading 00README.json, with fallbacks."""
    readme = source_dir / "00README.json"
    if readme.exists():
        data = json.loads(readme.read_text())
        for src in data.get("sources", []):
            if src.get("usage") == "toplevel":
                candidate = source_dir / src["filename"]
                if candidate.exists():
                    return candidate
        # toplevel entry pointed to a non-existent file — fall through to heuristics

    # Heuristic fallbacks
    for name in ("main.tex", "paper.tex", "manuscript.tex"):
        p = source_dir / name
        if p.exists():
            return p

    tex_files = [p for p in source_dir.glob("*.tex") if not p.name.startswith(".")]
    return tex_files[0] if len(tex_files) == 1 else None


def find_existing_truth_pdf(source_dir: Path) -> Path | None:
    """Return a PDF already present in the source directory, if any."""
    pdfs = [p for p in source_dir.glob("*.pdf") if not p.name.startswith(".")]
    return pdfs[0] if pdfs else None


# ─────────────────────────────────────────────────────────────────────────────
# Pipeline steps
# ─────────────────────────────────────────────────────────────────────────────

def ensure_bytetex(profile: str) -> Path:
    """Build bytetex if the release binary doesn't exist yet; return its path."""
    bin_path = REPO_ROOT / "target" / profile / "bytetex"
    if not bin_path.exists():
        flag = "--release" if profile == "release" else ""
        cmd = ["cargo", "build", "-p", "bytetex-cli"] + ([flag] if flag else [])
        print(f"  Building bytetex ({profile}) — this may take a minute ...", flush=True)
        subprocess.run(cmd, cwd=REPO_ROOT, check=True)
    return bin_path


def run_bytetex(
    bytetex_bin: Path, source_dir: Path, toplevel: Path
) -> tuple[Path | None, Path | None]:
    """Run bytetex convert; return (typ_path, warnings_path) or (None, None)."""
    result = subprocess.run(
        [str(bytetex_bin), "convert", toplevel.name],
        cwd=source_dir,
        capture_output=True,
        text=True,
    )
    typ_path = source_dir / (toplevel.stem + ".typ")
    warn_path = source_dir / (toplevel.stem + ".warnings.json")
    if not typ_path.exists() or typ_path.stat().st_size == 0:
        print(f"  [warn] bytetex produced no .typ output", file=sys.stderr)
        if result.stderr:
            print(f"         stderr: {result.stderr[:300]}", file=sys.stderr)
        return None, None
    return typ_path, (warn_path if warn_path.exists() else None)


def run_typst(typ_path: Path, out_pdf: Path) -> bool:
    """Compile .typ to PDF with typst; return True on success."""
    result = subprocess.run(
        ["typst", "compile", str(typ_path), str(out_pdf)],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"  [warn] typst compile failed (exit {result.returncode})", file=sys.stderr)
        if result.stderr:
            print(f"         {result.stderr[:400]}", file=sys.stderr)
        return False
    return out_pdf.exists() and out_pdf.stat().st_size > 0


def rasterize_pdf(pdf: Path, prefix: Path, dpi: int) -> list[Path]:
    """Rasterize a PDF to PNGs; return sorted list of produced PNGs."""
    prefix.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["pdftoppm", "-r", str(dpi), "-png", str(pdf), str(prefix)],
        check=True,
        capture_output=True,
    )
    pages = sorted(
        prefix.parent.glob(f"{prefix.name}-*.png"),
        key=lambda p: int(re.search(r"-(\d+)\.png$", p.name).group(1)),
    )
    return pages


def build_composite(
    truth_pages: list[Path],
    typst_pages: list[Path],
    out: Path,
    paper_id: str,
    cell_w: int = COMPOSITE_CELL_W,
    max_rows: int = MAX_COMPOSITE_PAGES,
) -> None:
    """Build a two-column composite PNG (truth left, typst right) for grading."""
    PADDING = 10
    HEADER_H = 26
    LABEL_H = 18
    GAP = 2

    n_rows = min(max_rows, max(len(truth_pages), len(typst_pages)))
    truncated = n_rows < max(len(truth_pages), len(typst_pages))

    # Derive cell height from the first available rasterized page
    first_page = truth_pages[0] if truth_pages else typst_pages[0]
    with Image.open(first_page) as sample:
        orig_w, orig_h = sample.size
    cell_h = int(cell_w * orig_h / orig_w)

    total_w = cell_w * 2 + PADDING * 3
    total_h = HEADER_H + PADDING + n_rows * (cell_h + LABEL_H + GAP)
    if truncated:
        total_h += 20

    canvas = Image.new("RGB", (total_w, total_h), "white")
    draw = ImageDraw.Draw(canvas)

    # Column headers
    draw.text((PADDING, 6), f"TRUTH  arxiv:{paper_id}", fill=(0, 0, 160))
    draw.text((PADDING * 2 + cell_w, 6), "TYPST  bytetex", fill=(140, 0, 0))
    draw.line([(0, HEADER_H), (total_w, HEADER_H)], fill=(200, 200, 200))

    y = HEADER_H + PADDING
    for row in range(n_rows):
        x_truth = PADDING
        x_typst = PADDING * 2 + cell_w

        # Truth column
        if row < len(truth_pages):
            with Image.open(truth_pages[row]) as pg:
                pg_resized = pg.convert("RGB").resize((cell_w, cell_h), Image.LANCZOS)
            canvas.paste(pg_resized, (x_truth, y))
        else:
            draw.rectangle([x_truth, y, x_truth + cell_w, y + cell_h], fill=(220, 220, 220))
            draw.text((x_truth + 4, y + cell_h // 2 - 6), "(no page)", fill=(100, 100, 100))

        # Typst column
        if row < len(typst_pages):
            with Image.open(typst_pages[row]) as pg:
                pg_resized = pg.convert("RGB").resize((cell_w, cell_h), Image.LANCZOS)
            canvas.paste(pg_resized, (x_typst, y))
        else:
            draw.rectangle([x_typst, y, x_typst + cell_w, y + cell_h], fill=(220, 220, 220))
            draw.text((x_typst + 4, y + cell_h // 2 - 6), "(no page)", fill=(100, 100, 100))

        # Page separator and label
        sep_y = y + cell_h
        draw.line([(0, sep_y), (total_w, sep_y)], fill=(180, 180, 180))
        draw.text((x_truth + 4, sep_y + 2), f"p.{row + 1}", fill=(120, 120, 120))
        draw.text((x_typst + 4, sep_y + 2), f"p.{row + 1}", fill=(120, 120, 120))

        y += cell_h + LABEL_H + GAP

    if truncated:
        truth_total = len(truth_pages)
        typst_total = len(typst_pages)
        draw.text(
            (PADDING, y + 4),
            f"[first {n_rows} of {max(truth_total, typst_total)} pages"
            f" | truth: {truth_total}  typst: {typst_total}]",
            fill=(100, 100, 100),
        )

    canvas.save(out, "PNG", optimize=True)


# ─────────────────────────────────────────────────────────────────────────────
# Warnings analysis
# ─────────────────────────────────────────────────────────────────────────────

def analyse_warnings(warn_path: Path | None) -> dict:
    if warn_path is None or not warn_path.exists():
        return {"total": 0, "by_severity": {}, "by_kind": {}}
    entries = json.loads(warn_path.read_text())
    by_sev: Counter = Counter()
    by_kind: Counter = Counter()
    for e in entries:
        by_sev[e.get("severity", "unknown")] += 1
        cat = e.get("category", {})
        by_kind[cat.get("kind", "unknown")] += 1
    return {
        "total": len(entries),
        "by_severity": dict(by_sev),
        "by_kind": dict(by_kind),
    }


# ─────────────────────────────────────────────────────────────────────────────
# Index (aggregate JSON)
# ─────────────────────────────────────────────────────────────────────────────

def load_index(path: Path) -> dict:
    if path.exists():
        return json.loads(path.read_text())
    return {"version": 1, "generated_at": _now(), "papers": {}}


def flush_index(index: dict, path: Path) -> None:
    index["generated_at"] = _now()
    path.write_text(json.dumps(index, indent=2) + "\n")


# ─────────────────────────────────────────────────────────────────────────────
# Per-paper orchestration
# ─────────────────────────────────────────────────────────────────────────────

def process_paper(
    arxiv_id: str,
    out_root: Path,
    session: requests.Session,
    bytetex_bin: Path,
    args: argparse.Namespace,
) -> dict:
    id_safe = arxiv_id.replace("/", "_")
    out_dir = out_root / id_safe
    composite_path = out_dir / "composite.png"

    if args.skip_existing and composite_path.exists():
        print(f"  [skip] composite already exists", flush=True)
        summary_path = out_dir / "summary.json"
        return json.loads(summary_path.read_text()) if summary_path.exists() else {
            "id": f"arxiv:{arxiv_id}", "status": "skipped"
        }

    out_dir.mkdir(parents=True, exist_ok=True)
    pages_dir = out_dir / "pages"
    pages_dir.mkdir(exist_ok=True)

    summary: dict = {
        "id": f"arxiv:{arxiv_id}",
        "generated_at": _now(),
        "toplevel_tex": None,
        "truth_pages": 0,
        "typst_pages": 0,
        "page_count_diff": None,
        "warnings": {},
        "status": "ok",
        "convert_ok": False,
        "typst_ok": False,
        "truth_source": None,
    }

    # 1. Locate source directory
    source_dir = find_source_dir(arxiv_id)
    if source_dir is None:
        summary["status"] = "no_source_dir"
        print(f"  [error] No source dir found under corpus/online/arxiv/", file=sys.stderr)
        return summary

    toplevel = find_toplevel_tex(source_dir)
    if toplevel is None:
        summary["status"] = "no_toplevel"
        print(f"  [error] Cannot identify entry .tex in {source_dir}", file=sys.stderr)
        return summary

    summary["toplevel_tex"] = toplevel.name
    print(f"  toplevel: {toplevel.name}", flush=True)

    # 2. Acquire truth PDF — always use the arXiv canonical PDF.
    # Bundled source PDFs are unreliable (often partial/draft artifacts).
    truth_dest = out_dir / "truth.pdf"
    if truth_dest.exists() and truth_dest.stat().st_size > 10_000:
        # Already downloaded in a previous run — reuse.
        summary["truth_source"] = "cached"
        print(f"  truth PDF: cached ({truth_dest.stat().st_size // 1024} KB)", flush=True)
    elif args.no_truth_download:
        summary["status"] = "no_truth_pdf"
        print(f"  [error] No cached truth PDF and --no-truth-download set", file=sys.stderr)
        return summary
    else:
        url = ARXIV_PDF_URL.format(id=arxiv_id)
        print(f"  downloading truth PDF from arXiv ...", flush=True)
        r = fetch(session, url, stream=True)
        if r.status_code != 200:
            summary["status"] = "truth_pdf_download_failed"
            print(f"  [error] arXiv returned HTTP {r.status_code}", file=sys.stderr)
            return summary
        with open(truth_dest, "wb") as f:
            for chunk in r.iter_content(65536):
                f.write(chunk)
        sleep_politely(args.delay)
        summary["truth_source"] = "arxiv_download"
        kb = truth_dest.stat().st_size // 1024
        print(f"  truth PDF: downloaded ({kb} KB)", flush=True)

    # 3. Convert with bytetex
    print(f"  bytetex convert ...", flush=True)
    typ_path, warn_path = run_bytetex(bytetex_bin, source_dir, toplevel)
    if typ_path is None:
        summary["status"] = "convert_failed"
        return summary
    summary["convert_ok"] = True
    summary["warnings"] = analyse_warnings(warn_path)
    if warn_path:
        shutil.copy2(warn_path, out_dir / "typst.warnings.json")

    # 4. Compile with typst
    typst_pdf = out_dir / "typst.pdf"
    print(f"  typst compile ...", flush=True)
    if not run_typst(typ_path, typst_pdf):
        summary["status"] = "typst_compile_failed"
        return summary
    summary["typst_ok"] = True

    # 5. Rasterize both PDFs
    print(f"  rasterizing ...", flush=True)
    truth_pages = rasterize_pdf(truth_dest, pages_dir / "truth", args.rasterize_dpi)
    typst_pages = rasterize_pdf(typst_pdf, pages_dir / "typst", args.rasterize_dpi)
    summary["truth_pages"] = len(truth_pages)
    summary["typst_pages"] = len(typst_pages)
    summary["page_count_diff"] = len(typst_pages) - len(truth_pages)
    print(f"  pages — truth: {len(truth_pages)}, typst: {len(typst_pages)}", flush=True)

    # 6. Build composite
    print(f"  building composite ...", flush=True)
    build_composite(truth_pages, typst_pages, composite_path, arxiv_id)
    kb = composite_path.stat().st_size // 1024
    print(f"  composite.png: {kb} KB", flush=True)

    # 7. Write per-paper summary
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument(
        "--papers", nargs="+", default=DEFAULT_PAPERS, metavar="ID",
        help="arXiv IDs to process (default: 5-paper diversity set)",
    )
    p.add_argument(
        "--out", type=Path, default=Path("tests/visual"), metavar="PATH",
        help="output directory (default: tests/visual)",
    )
    p.add_argument(
        "--skip-existing", action="store_true",
        help="skip papers whose composite.png already exists",
    )
    p.add_argument(
        "--release", dest="profile", action="store_const", const="release", default="release",
        help="build bytetex in release mode (default)",
    )
    p.add_argument(
        "--debug", dest="profile", action="store_const", const="debug",
        help="build bytetex in debug mode (faster build, slower binary)",
    )
    p.add_argument(
        "--rasterize-dpi", type=int, default=RASTERIZE_DPI, metavar="DPI",
        help=f"pdftoppm DPI for rasterization (default: {RASTERIZE_DPI})",
    )
    p.add_argument(
        "--no-truth-download", action="store_true",
        help="error if truth PDF is not already on disk",
    )
    p.add_argument(
        "--delay", type=float, default=ARXIV_MIN_DELAY, metavar="SEC",
        help=f"polite delay between arXiv PDF downloads (default: {ARXIV_MIN_DELAY}s)",
    )
    p.add_argument("--user-agent", default=DEFAULT_UA, metavar="UA")
    args = p.parse_args()

    out = args.out if args.out.is_absolute() else (REPO_ROOT / args.out)
    out.mkdir(parents=True, exist_ok=True)
    index_path = out / "index.json"
    index = load_index(index_path)

    session = make_session(args.user_agent)
    bytetex_bin = ensure_bytetex(args.profile)

    for arxiv_id in args.papers:
        print(f"\n=== {arxiv_id} ===", flush=True)
        try:
            summary = process_paper(arxiv_id, out, session, bytetex_bin, args)
        except Exception as exc:
            import traceback
            print(f"  [fatal] {exc}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
            summary = {"id": f"arxiv:{arxiv_id}", "status": "exception", "error": str(exc)}

        index["papers"][arxiv_id] = {
            "status": summary.get("status", "unknown"),
            "convert_ok": summary.get("convert_ok", False),
            "typst_ok": summary.get("typst_ok", False),
            "truth_pages": summary.get("truth_pages", 0),
            "typst_pages": summary.get("typst_pages", 0),
            "page_count_diff": summary.get("page_count_diff"),
            "warnings_total": summary.get("warnings", {}).get("total", 0),
            "composite": str(out / arxiv_id.replace("/", "_") / "composite.png")
                if summary.get("typst_ok") else None,
        }
        flush_index(index, index_path)
        print(f"  → status: {summary.get('status')}", flush=True)

    ok_count = sum(1 for v in index["papers"].values() if v["status"] == "ok")
    print(f"\nDone: {ok_count}/{len(args.papers)} fully processed.")
    print(f"Index: {index_path}")
    print("Next: ask the agent to read each composite.png and write tests/visual/report.md")


if __name__ == "__main__":
    main()
