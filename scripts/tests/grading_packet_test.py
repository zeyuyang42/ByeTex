#!/usr/bin/env python3
"""Unit tests for the front-matter crop + grading packet pieces in visual_test.py:
  - crop_front_matter: full-width, top-fraction crop of a page raster
  - detect_doc_class: documentclass detection (incl. conference packages)
  - build_grading_packet: per-paper grading_packet.json assembly

Run: uv run --with Pillow python scripts/tests/grading_packet_test.py
"""
import json
import shutil
import sys
import tempfile
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import visual_test as vt  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


def _png(path: Path, w: int, h: int, fill: int = 200) -> Path:
    Image.new("L", (w, h), color=fill).save(path)
    return path


# ── crop_front_matter ────────────────────────────────────────────────────────
with tempfile.TemporaryDirectory() as td:
    d = Path(td)
    page = _png(d / "page.png", 400, 1000)
    out = vt.crop_front_matter(page, d / "fm.png", top_frac=0.40)
    check(out.exists(), "crop_front_matter writes the output file")
    with Image.open(out) as im:
        cw, ch = im.size
    check(cw == 400, f"crop keeps full width (got {cw})")
    check(ch == int(1000 * 0.40), f"crop keeps top 40% height (got {ch})")


# ── detect_doc_class ───────────────────────────────────────────────────────────
with tempfile.TemporaryDirectory() as td:
    d = Path(td)

    def _cls(text: str) -> str | None:
        f = d / "main.tex"
        f.write_text(text)
        return vt.detect_doc_class(f)

    check(_cls(r"\documentclass{article}" + "\n" + r"\usepackage{neurips_2026}") == "neurips",
          "article + neurips_* package -> 'neurips'")
    # Regression: the conference package may carry a path prefix (corpus
    # 2605.22507 uses \usepackage[main,preprint]{style/neurips_2026}).
    check(_cls(r"\documentclass{article}" + "\n" + r"\usepackage[main,preprint]{style/neurips_2026}") == "neurips",
          "article + path-prefixed neurips package -> 'neurips'")
    check(_cls(r"\documentclass{article}" + "\n" + r"\usepackage{icml2026}") == "icml",
          "article + icml package -> 'icml'")
    check(_cls(r"\documentclass[conference]{IEEEtran}") == "ieeetran",
          "IEEEtran -> 'ieeetran'")
    check(_cls(r"\documentclass[sigconf]{acmart}") == "acmart",
          "acmart -> 'acmart'")
    check(_cls(r"\documentclass{article}") == "article",
          "plain article -> 'article'")
    check(vt.detect_doc_class(d / "missing.tex") is None,
          "unreadable file -> None")
    check(_cls("no class here at all") is None,
          "no \\documentclass -> None")


# ── build_grading_packet ───────────────────────────────────────────────────────
with tempfile.TemporaryDirectory() as td:
    out_dir = Path(td) / "2605.12345"
    pages_dir = out_dir / "pages"
    pages_dir.mkdir(parents=True)
    _png(pages_dir / "truth-1.png", 60, 80)
    _png(pages_dir / "typst-1.png", 60, 80)
    # Regression: high-DPI front-matter intermediates share the `<side>-*` glob;
    # they must NOT be paired as standard pages (would collide on page 1).
    _png(pages_dir / "truth-fm-1.png", 120, 160)
    _png(pages_dir / "typst-fm-1.png", 120, 160)

    summary = {
        "id": "arxiv:2605.12345",
        "truth_source": "arxiv_download",
        "structure": {"word_recall": 0.9, "heading_recall": 1.0},
        "warnings": {"total": 3},
    }
    fm = {"truth": "pages/frontmatter-truth.png", "typst": "pages/frontmatter-typst.png"}
    packet_path = vt.build_grading_packet(out_dir, summary, fm, "neurips")
    check(packet_path.exists(), "build_grading_packet writes grading_packet.json")
    pkt = json.loads(packet_path.read_text())

    check(pkt["id"] == "arxiv:2605.12345", "packet carries the id")
    check(pkt["detected_class"] == "neurips", "packet records detected_class")
    check(pkt["truth_source"] == "arxiv_download", "packet records truth_source")
    check(pkt["front_matter"] == fm, "packet carries front_matter crop paths")
    check(pkt["composite"] == "composite.png", "packet points at composite.png")
    check(pkt["rubric"] == "docs/fidelity-rubric.md", "packet points at the rubric")
    # structure/warnings inlined, not paths
    check(pkt["structure"] == {"word_recall": 0.9, "heading_recall": 1.0},
          "structure is inlined verbatim")
    check(pkt["warnings"] == {"total": 3}, "warnings are inlined verbatim")
    # pages: the on-disk pair, real filenames, page 1
    check(isinstance(pkt["pages"], list) and len(pkt["pages"]) == 1,
          "pages lists the single on-disk pair")
    pg = pkt["pages"][0]
    check(pg["page"] == 1, "page number derived from filename")
    check(pg["truth"] == "pages/truth-1.png", f"real truth filename (got {pg.get('truth')})")
    check(pg["typst"] == "pages/typst-1.png", f"real typst filename (got {pg.get('typst')})")


if fails:
    print(f"\nTEST FAILED ({len(fails)} assertion(s))")
    sys.exit(1)
print("\nTEST PASSED")
