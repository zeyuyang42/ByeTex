#!/usr/bin/env python3
"""Build a single browsable HTML gallery of visual-test composites.

Reads the per-paper render output under a `tests/visual/` tree (produced by
`scripts/visual_test.py`) and emits `<out>/gallery.html`: a summary metrics table
plus one section per document type, each embedding that paper's side-by-side
`composite.png` (LaTeX truth ↔ byetex Typst). Read-only over the visual dir.

Usage:
    python scripts/build_gallery.py --out tests/visual <id> [<id> ...]

Each id is rendered in the order given, grouped by an inferred document type. The
composites are referenced by *relative* path, so open the emitted gallery.html
from inside the --out directory (its siblings are the <id>/ dirs).
"""
from __future__ import annotations

import argparse
import html
import json
from pathlib import Path

# Doc-type inference from the corpus id. arXiv ids (NNNN.NNNNN) are papers.
DOC_TYPE_ORDER = ["Paper", "Beamer", "Book", "Thesis", "Report", "Other"]


def infer_doc_type(paper_id: str) -> str:
    low = paper_id.lower()
    if "beamer" in low or low == "beamer-demo" or "metropolis" in low or "mtheme" in low:
        return "Beamer"
    if "thesis" in low:
        return "Thesis"
    if "report" in low:
        return "Report"
    if "book" in low or "kaobook" in low or "memoir" in low:
        return "Book"
    return "Paper"


def detected_class(visual_dir: Path, paper_id: str) -> str:
    packet = visual_dir / paper_id / "grading_packet.json"
    if packet.exists():
        try:
            return json.loads(packet.read_text()).get("detected_class") or "—"
        except (json.JSONDecodeError, OSError):
            pass
    return "—"


def page_ratio(entry: dict) -> str:
    tp, yp = entry.get("truth_pages"), entry.get("typst_pages")
    if tp and yp:
        return f"{yp / tp:.2f}"
    return "—"


def fmt(v) -> str:
    return f"{v:.3f}" if isinstance(v, (int, float)) else "—"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, default=Path("tests/visual"),
                    help="visual-test output dir (default: tests/visual)")
    ap.add_argument("ids", nargs="+", help="paper/corpus ids to include")
    args = ap.parse_args()

    visual_dir: Path = args.out
    index_path = visual_dir / "index.json"
    index = json.loads(index_path.read_text()).get("papers", {}) if index_path.exists() else {}

    # Collect a row per id, ordered by doc type then input order.
    rows = []
    for paper_id in args.ids:
        entry = index.get(paper_id, {})
        rows.append({
            "id": paper_id,
            "doc_type": infer_doc_type(paper_id),
            "klass": detected_class(visual_dir, paper_id),
            "status": entry.get("status", "missing"),
            "truth_pages": entry.get("truth_pages"),
            "typst_pages": entry.get("typst_pages"),
            "page_ratio": page_ratio(entry),
            "word_recall": entry.get("word_recall"),
            "heading_recall": entry.get("heading_recall"),
            "mean_ssim": entry.get("mean_ssim"),
            "has_composite": (visual_dir / paper_id / "composite.png").exists(),
        })
    rows.sort(key=lambda r: (DOC_TYPE_ORDER.index(r["doc_type"]), args.ids.index(r["id"])))

    fidelity = json.loads(index_path.read_text()).get("fidelity_score") if index_path.exists() else None

    # ---- Build HTML ----
    css = """
    body { font: 14px/1.5 -apple-system, system-ui, sans-serif; margin: 2rem; color: #222; }
    h1 { margin-bottom: .25rem; } .sub { color: #666; margin-top: 0; }
    table { border-collapse: collapse; margin: 1rem 0 2rem; font-size: 13px; }
    th, td { border: 1px solid #ddd; padding: 4px 8px; text-align: right; }
    th { background: #f4f6f8; } td.l, th.l { text-align: left; }
    .ok { color: #137333; } .warn { color: #b06000; } .fail { color: #b00020; }
    h2 { border-bottom: 2px solid #eee; padding-bottom: .25rem; margin-top: 2.5rem; }
    .card { margin: 1rem 0 2rem; }
    .card .cap { color: #555; margin: .25rem 0 .5rem; }
    .card img { max-width: 100%; border: 1px solid #ccc; box-shadow: 0 1px 4px rgba(0,0,0,.1); }
    .note { background: #fff8e1; border-left: 3px solid #b06000; padding: .25rem .6rem; display: inline-block; }
    code { background: #f0f0f0; padding: 0 4px; border-radius: 3px; }
    """

    def status_class(s: str) -> str:
        if s == "ok":
            return "ok"
        if "fail" in s:
            return "fail"
        return "warn"

    out = []
    out.append("<!doctype html><html><head><meta charset='utf-8'>")
    out.append("<title>ByeTex — cross-doc-type render gallery</title>")
    out.append(f"<style>{css}</style></head><body>")
    out.append("<h1>ByeTex render gallery</h1>")
    sub = "Side-by-side <b>LaTeX truth</b> (left) ↔ <b>byetex Typst</b> (right), across document types."
    if fidelity is not None:
        sub += f" &nbsp;·&nbsp; corpus fidelity_score <b>{fidelity:.3f}</b>"
    out.append(f"<p class='sub'>{sub}</p>")

    # Summary table
    out.append("<table><thead><tr>"
               "<th class='l'>Type</th><th class='l'>id</th><th class='l'>class</th>"
               "<th class='l'>status</th><th>truth pp</th><th>typst pp</th>"
               "<th>page ratio</th><th>word recall</th><th>heading recall</th><th>SSIM</th>"
               "</tr></thead><tbody>")
    for r in rows:
        out.append(
            "<tr>"
            f"<td class='l'>{html.escape(r['doc_type'])}</td>"
            f"<td class='l'><a href='#{html.escape(r['id'])}'>{html.escape(r['id'])}</a></td>"
            f"<td class='l'>{html.escape(str(r['klass']))}</td>"
            f"<td class='l {status_class(r['status'])}'>{html.escape(r['status'])}</td>"
            f"<td>{r['truth_pages'] if r['truth_pages'] is not None else '—'}</td>"
            f"<td>{r['typst_pages'] if r['typst_pages'] is not None else '—'}</td>"
            f"<td>{r['page_ratio']}</td>"
            f"<td>{fmt(r['word_recall'])}</td>"
            f"<td>{fmt(r['heading_recall'])}</td>"
            f"<td>{fmt(r['mean_ssim'])}</td>"
            "</tr>"
        )
    out.append("</tbody></table>")

    # Per-type sections with composites
    last_type = None
    for r in rows:
        if r["doc_type"] != last_type:
            out.append(f"<h2>{html.escape(r['doc_type'])}</h2>")
            last_type = r["doc_type"]
        out.append(f"<div class='card' id='{html.escape(r['id'])}'>")
        out.append(f"<div class='cap'><b>{html.escape(r['id'])}</b> "
                   f"<code>{html.escape(str(r['klass']))}</code> — status "
                   f"<span class='{status_class(r['status'])}'>{html.escape(r['status'])}</span></div>")
        if r["status"] == "truth_render_failed":
            out.append("<div class='note'>truth render failed (tectonic couldn't build this source) — "
                       "showing <b>byetex output only</b>, no truth reference.</div>")
        if r["has_composite"]:
            out.append(f"<div><img loading='lazy' src='{html.escape(r['id'])}/composite.png' "
                       f"alt='{html.escape(r['id'])} composite'></div>")
        else:
            out.append("<div class='note'>no composite.png found for this id.</div>")
        out.append("</div>")

    out.append("</body></html>")

    gallery = visual_dir / "gallery.html"
    gallery.write_text("\n".join(out))
    print(f"wrote {gallery} ({len(rows)} docs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
