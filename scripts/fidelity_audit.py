#!/usr/bin/env python3
"""Non-visual fidelity-gap audit for ByeTex (LaTeX -> Typst).

Ranks fidelity gaps across the corpus with ZERO vision / PDF rendering, using
two complementary signals:

  A. Warning aggregation — run `byetex convert` per paper and aggregate every
     warnings.json entry by category kind + command name. Surfaces gaps the
     converter already flags (e.g. `\\resizebox` UnsupportedCommand, tikz).

  B. Silent-gap source scan — regex the resolved project source (toplevel +
     transitive \\input/\\include/\\subfile) for constructs that LOSE fidelity
     with NO warning: dropped column widths, `\\textcolor`, `\\vspace`, custom
     `\\item[..]` labels, theorem `[Note]` args, etc.

Output: a ranked Markdown report (docs/fidelity-nonvisual-audit.md) + a
machine-readable JSON sidecar. The ranking drives which emitter gaps to fix.

Env overrides (match scripts/corpus_sweep.sh):
  BYETEX_BIN         path to the byetex binary (skips the cargo build)
  BYETEX_CORPUS_DIR  corpus root to audit instead of <repo>/corpus
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CORPUS_DIR = Path(os.environ.get("BYETEX_CORPUS_DIR", REPO_ROOT / "corpus"))
DOCS_OUT = REPO_ROOT / "docs" / "fidelity-nonvisual-audit.md"
JSON_OUT = REPO_ROOT / "docs" / "fidelity-nonvisual-audit.json"

# ── source helpers (mirrors scripts/visual_test.py) ──────────────────────────

_COMMENT_RE = re.compile(r"(?<!\\)%.*")
_INPUT_RE = re.compile(r"\\(?:input|include|subfile)\s*\{([^}]+)\}")


def strip_comments(tex: str) -> str:
    return "\n".join(_COMMENT_RE.sub("", line) for line in tex.splitlines())


def find_source_dir(arxiv_id: str) -> Path | None:
    cand = CORPUS_DIR / arxiv_id / "source"
    return cand if cand.exists() else None


def find_toplevel_tex(source_dir: Path) -> Path | None:
    readme = source_dir / "00README.json"
    if readme.exists():
        try:
            data = json.loads(readme.read_text())
            for src in data.get("sources", []):
                if src.get("usage") == "toplevel":
                    cand = source_dir / src["filename"]
                    if cand.exists():
                        return cand
        except (OSError, json.JSONDecodeError):
            pass
    for name in ("main.tex", "paper.tex", "manuscript.tex"):
        p = source_dir / name
        if p.exists():
            return p
    tex = [p for p in source_dir.glob("*.tex") if not p.name.startswith(".")]
    return tex[0] if len(tex) == 1 else None


def collect_project_source(toplevel: Path) -> str:
    """Toplevel + transitive \\input/\\include/\\subfile, comments stripped."""
    root = toplevel.parent
    visited: set[Path] = set()
    parts: list[str] = []

    def resolve(raw: str, base: Path) -> Path | None:
        raw = raw.strip()
        for cand_dir in (base, root):
            for name in (raw, raw + ".tex"):
                p = cand_dir / name
                if p.is_file():
                    return p.resolve()
        return None

    def walk(path: Path) -> None:
        rp = path.resolve()
        if rp in visited or not rp.is_file():
            return
        visited.add(rp)
        try:
            text = rp.read_text(encoding="utf-8", errors="replace")
        except OSError:
            return
        stripped = strip_comments(text)
        parts.append(stripped)
        for m in _INPUT_RE.finditer(stripped):
            child = resolve(m.group(1), rp.parent)
            if child is not None:
                walk(child)

    walk(toplevel)
    return "\n".join(parts)


# ── silent-gap patterns (fidelity loss WITHOUT a warning) ────────────────────
# label -> (cluster, counter(text) -> int)

def _count(rx: re.Pattern) -> "callable":
    return lambda s: len(rx.findall(s))


_TABULAR_RE = re.compile(r"\\begin\{(?:tabular[*x]?|array)\}\s*(?:\[[^\]]*\])?\s*(?:\{[^}]*\})?\{([^}]*)\}")


def _colspec_count(needle_rx: re.Pattern):
    """Count `needle` only inside tabular/array column-spec braces."""
    def counter(s: str) -> int:
        return sum(len(needle_rx.findall(cs)) for cs in _TABULAR_RE.findall(s))
    return counter


SILENT_GAPS: dict[str, tuple[str, object]] = {
    # Float sizing & tables
    r"\resizebox": ("float-sizing/tables", _count(re.compile(r"\\resizebox\b"))),
    "p/m/b column widths": ("float-sizing/tables", _colspec_count(re.compile(r"[pmb]\{"))),
    r">{}/<{}/@{} col decorators": ("float-sizing/tables", _colspec_count(re.compile(r"[><@!]\{"))),
    r"\cmidrule": ("float-sizing/tables", _count(re.compile(r"\\cmidrule\b"))),
    # Color & spacing
    r"\textcolor": ("color/spacing", _count(re.compile(r"\\textcolor\b"))),
    r"\colorbox/\fcolorbox": ("color/spacing", _count(re.compile(r"\\f?colorbox\b"))),
    r"\definecolor": ("color/spacing", _count(re.compile(r"\\definecolor\b"))),
    r"\vspace/\hspace": ("color/spacing", _count(re.compile(r"\\[vh]space\*?\b"))),
    r"\smallskip/\medskip/\bigskip": ("color/spacing", _count(re.compile(r"\\(?:small|med|big)skip\b"))),
    # Lists & theorems
    r"\item[custom-label]": ("lists/theorems", _count(re.compile(r"\\item\s*\["))),
    "enumerate[style] (enumitem)": ("lists/theorems", _count(re.compile(r"\\begin\{enumerate\}\s*\["))),
    r"\renewcommand{\labelenum*}": ("lists/theorems", _count(re.compile(r"\\renewcommand\s*\{\\labelenum"))),
    "theorem-env [Note]": ("lists/theorems", _count(re.compile(
        r"\\begin\{(?:theorem|lemma|corollary|proposition|definition|remark|example|claim|conjecture|proof)\}\s*\["))),
}


def ensure_bin() -> Path:
    env = os.environ.get("BYETEX_BIN")
    if env:
        return Path(env)
    bin_path = REPO_ROOT / "target" / "release" / "byetex"
    if not bin_path.exists():
        print("Building byetex (release) ...", file=sys.stderr)
        subprocess.run(
            ["cargo", "build", "--release", "-p", "byetex-cli"],
            cwd=REPO_ROOT, check=True,
        )
    return bin_path


def convert_warnings(byetex: Path, toplevel: Path, out_dir: Path) -> list[dict]:
    """Run `byetex convert --project` and return the warnings.json entries."""
    proj = out_dir / "proj"
    subprocess.run(
        [str(byetex), "convert", "--project", toplevel.name,
         "--project-out", str(proj), "--force", "--no-brief"],
        cwd=toplevel.parent, capture_output=True, text=True,
    )
    wj = proj / "warnings.json"
    if not wj.exists():
        return []
    try:
        data = json.loads(wj.read_text())
    except (OSError, json.JSONDecodeError):
        return []
    return data if isinstance(data, list) else data.get("warnings", [])


def warning_key(entry: dict) -> tuple[str, str]:
    cat = entry.get("category", {})
    kind = cat.get("kind", "unknown") if isinstance(cat, dict) else str(cat)
    name = ""
    if isinstance(cat, dict):
        name = cat.get("name") or cat.get("command") or cat.get("env") or cat.get("package") or ""
    return kind, name


def main() -> int:
    byetex = ensure_bin()
    paper_ids = sorted(
        p.name for p in CORPUS_DIR.iterdir()
        if p.is_dir() and re.fullmatch(r"\d{4}\.\d{4,6}", p.name)
    )
    if not paper_ids:
        print(f"No papers under {CORPUS_DIR}", file=sys.stderr)
        return 1

    warn_papers: dict[tuple[str, str], set[str]] = defaultdict(set)
    warn_total: dict[tuple[str, str], int] = defaultdict(int)
    silent_papers: dict[str, set[str]] = defaultdict(set)
    silent_total: dict[str, int] = defaultdict(int)
    audited = skipped = 0

    with tempfile.TemporaryDirectory(dir=REPO_ROOT, prefix=".audit-") as tmp:
        tmp_path = Path(tmp)
        for pid in paper_ids:
            src = find_source_dir(pid)
            top = find_toplevel_tex(src) if src else None
            if top is None:
                skipped += 1
                continue
            audited += 1
            # A. warnings
            for entry in convert_warnings(byetex, top, tmp_path / pid):
                key = warning_key(entry)
                warn_papers[key].add(pid)
                warn_total[key] += 1
            # B. silent source scan
            source = collect_project_source(top)
            for label, (_cluster, counter) in SILENT_GAPS.items():
                n = counter(source)
                if n:
                    silent_papers[label].add(pid)
                    silent_total[label] += n
            print(f"  audited {pid}", file=sys.stderr)

    report = render(paper_ids, audited, skipped,
                    warn_papers, warn_total, silent_papers, silent_total)
    DOCS_OUT.parent.mkdir(parents=True, exist_ok=True)
    DOCS_OUT.write_text(report)

    js = {
        "corpus_dir": str(CORPUS_DIR),
        "papers_total": len(paper_ids),
        "papers_audited": audited,
        "papers_skipped": skipped,
        "warnings": [
            {"kind": k, "name": n, "papers": len(warn_papers[(k, n)]), "occurrences": warn_total[(k, n)]}
            for (k, n) in sorted(warn_total, key=lambda x: (-len(warn_papers[x]), -warn_total[x]))
        ],
        "silent_gaps": [
            {"gap": label, "cluster": SILENT_GAPS[label][0],
             "papers": len(silent_papers[label]), "occurrences": silent_total[label]}
            for label in sorted(silent_total, key=lambda l: (-len(silent_papers[l]), -silent_total[l]))
        ],
    }
    JSON_OUT.write_text(json.dumps(js, indent=2) + "\n")
    print(f"\nWrote {DOCS_OUT.relative_to(REPO_ROOT)} and {JSON_OUT.relative_to(REPO_ROOT)}", file=sys.stderr)
    print(f"audited={audited} skipped={skipped}", file=sys.stderr)
    return 0


def render(paper_ids, audited, skipped, warn_papers, warn_total, silent_papers, silent_total) -> str:
    out = ["# Non-visual fidelity audit", "",
           f"Corpus: `{CORPUS_DIR}` — {len(paper_ids)} papers ({audited} audited, {skipped} skipped).",
           "",
           "Generated by `scripts/fidelity_audit.py` (no vision/PDF). Two signals:",
           "**Silent gaps** (fidelity lost with NO warning, ranked by #papers) and",
           "**Warnings** (the converter already flags these).", ""]

    out += ["## Silent gaps (in-scope clusters)", "",
            "| gap | cluster | papers | occurrences |", "| --- | --- | --: | --: |"]
    for label in sorted(silent_total, key=lambda l: (-len(silent_papers[l]), -silent_total[l])):
        cluster = SILENT_GAPS[label][0]
        out.append(f"| `{label}` | {cluster} | {len(silent_papers[label])} | {silent_total[label]} |")
    out.append("")

    ranked = sorted(warn_total, key=lambda x: (-len(warn_papers[x]), -warn_total[x]))
    TOP = 40
    out += [f"## Warnings aggregated (top {TOP} of {len(ranked)}; full list in the .json)", "",
            "| kind | name | papers | occurrences |", "| --- | --- | --: | --: |"]
    for (kind, name) in ranked[:TOP]:
        out.append(f"| {kind} | `{name}` | {len(warn_papers[(kind, name)])} | {warn_total[(kind, name)]} |")
    out.append("")
    return "\n".join(out)


if __name__ == "__main__":
    sys.exit(main())
