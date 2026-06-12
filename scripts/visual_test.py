#!/usr/bin/env python3
"""
ByeTex visual regression test — produces side-by-side composites for agent grading.

For each arXiv paper:
  1. Reads entry .tex from source/00README.json
  2. Downloads the arXiv canonical PDF (or uses the one bundled in the source dir)
  3. Runs byetex convert → .typ + .warnings.json
  4. Compiles with typst → typst.pdf
  5. Rasterizes both PDFs with pdftoppm
  6. Stacks pages into a side-by-side composite.png for agent visual grading

Usage (add --with numpy --with scikit-image to enable the per-page SSIM
metric; it degrades to mean_ssim=null if those aren't installed):
    uv run --with requests --with Pillow --with numpy --with scikit-image \
        python scripts/visual_test.py
    uv run --with requests --with Pillow --with numpy --with scikit-image \
        python scripts/visual_test.py --papers 2605.22507
    uv run --with requests --with Pillow python scripts/visual_test.py --skip-existing
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
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
CORPUS_DIR = REPO_ROOT / "corpus"
MANIFEST_PATH = CORPUS_DIR / "manifest.json"

ARXIV_PDF_URL = "https://arxiv.org/pdf/{id}"
ARXIV_MIN_DELAY = 3.0
DEFAULT_UA = (
    "ByeTex-Harvester/0.1 (+https://github.com/zeyuyang42/ByeTex; "
    "research/testing use only)"
)


def load_pinned_ids() -> list[str]:
    """Return IDs marked pinned:true in corpus/manifest.json."""
    if not MANIFEST_PATH.exists():
        return []
    data = json.loads(MANIFEST_PATH.read_text())
    return [p["id"] for p in data.get("papers", []) if p.get("pinned")]


DEFAULT_PAPERS = load_pinned_ids()

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
    """Find the source/ directory for an arXiv ID under corpus/."""
    candidate = CORPUS_DIR / arxiv_id / "source"
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


_INPUT_RE = re.compile(r"\\(?:input|include|subfile)\s*\{([^}]+)\}")


def collect_project_source(toplevel: Path) -> str:
    """Concatenate the toplevel .tex and every file it pulls in via
    `\\input`/`\\include`/`\\subfile`, transitively. Corpus papers keep their
    sections in separate files (0 in the toplevel, 7-23 includes), so the
    source-anchored truth metrics (source_headings / source_float_counts) need
    the whole project, not just the entry file. Resolution mirrors byetex:
    relative to the including file's dir, then the project root. Best-effort —
    unreadable/missing includes are skipped; visited files aren't re-read."""
    root = toplevel.parent
    visited: set[Path] = set()
    parts: list[str] = []

    def resolve(raw: str, base: Path) -> Path | None:
        raw = raw.strip()
        for cand_dir in (base, root):
            for name in (raw, raw + ".tex"):
                p = (cand_dir / name)
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
        parts.append(text)
        for m in _INPUT_RE.finditer(_strip_comments(text)):
            child = resolve(m.group(1), rp.parent)
            if child is not None:
                walk(child)

    walk(toplevel)
    return "\n".join(parts)


# ─────────────────────────────────────────────────────────────────────────────
# Pipeline steps
# ─────────────────────────────────────────────────────────────────────────────

def ensure_byetex(profile: str) -> Path:
    """Build byetex if the release binary doesn't exist yet; return its path."""
    bin_path = REPO_ROOT / "target" / profile / "byetex"
    if not bin_path.exists():
        flag = "--release" if profile == "release" else ""
        cmd = ["cargo", "build", "-p", "byetex-cli"] + ([flag] if flag else [])
        print(f"  Building byetex ({profile}) — this may take a minute ...", flush=True)
        subprocess.run(cmd, cwd=REPO_ROOT, check=True)
    return bin_path


def run_byetex(
    byetex_bin: Path, source_dir: Path, toplevel: Path
) -> tuple[Path | None, Path | None]:
    """Run `byetex convert --project` and return (main.typ, warnings.json)
    from the generated project, or (None, None).

    Project mode (not flat `convert`) is what the corpus papers actually use:
    it pre-scans sibling files for macros and referenced labels, preprocesses
    `.bib`, and copies assets next to `main.typ`. Flat convert would miss the
    cross-file pre-scans and mis-resolve assets, under-representing quality.
    """
    proj_rel = f"{toplevel.stem}.typst-project"
    proj_dir = source_dir / proj_rel
    shutil.rmtree(proj_dir, ignore_errors=True)
    result = subprocess.run(
        [
            str(byetex_bin), "convert", "--project", toplevel.name,
            "--project-out", proj_rel, "--force", "--no-brief",
        ],
        cwd=source_dir,
        capture_output=True,
        text=True,
    )
    typ_path = proj_dir / "main.typ"
    warn_path = proj_dir / "warnings.json"
    if not typ_path.exists() or typ_path.stat().st_size == 0:
        print(f"  [warn] byetex --project produced no main.typ", file=sys.stderr)
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


# ─────────────────────────────────────────────────────────────────────────────
# Tectonic reference renderer (local LaTeX → PDF "truth")
#
# Lets us render the *original* LaTeX to PDF locally instead of relying on an
# arXiv canonical download — so round-trip comparison works for arbitrary
# inputs and offline. Mirrors the `byetex doctor` shell-out: skip cleanly when
# tectonic is absent. BYETEX_TECTONIC_BIN overrides the binary (tests / custom
# installs), matching the Rust side.
# ─────────────────────────────────────────────────────────────────────────────

def tectonic_bin() -> str:
    return os.environ.get("BYETEX_TECTONIC_BIN", "tectonic")


def tectonic_available() -> bool:
    try:
        return subprocess.run(
            [tectonic_bin(), "--version"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        ).returncode == 0
    except FileNotFoundError:
        return False


def render_reference_tectonic(toplevel: Path, out_pdf: Path) -> bool:
    """Render a LaTeX source to PDF with tectonic; return True on success.

    The scratch outputs land in a tempdir anchored inside the source's own
    directory (kept out of the system temp), and the produced PDF is copied
    to `out_pdf`.
    """
    # Resolve to absolute so --outdir is independent of the subprocess cwd
    # (we run with cwd=src_dir so \input/\include resolve like the source).
    src_dir = toplevel.parent.resolve()
    with tempfile.TemporaryDirectory(dir=src_dir, prefix=".tectonic-out-") as tmp:
        result = subprocess.run(
            [tectonic_bin(), "--outdir", str(Path(tmp)), "--keep-logs", toplevel.name],
            cwd=src_dir, capture_output=True, text=True,
        )
        produced = Path(tmp) / (toplevel.stem + ".pdf")
        if result.returncode != 0 or not produced.exists():
            print(f"  [warn] tectonic render failed (exit {result.returncode})", file=sys.stderr)
            if result.stderr:
                print(f"         {result.stderr[-400:]}", file=sys.stderr)
            return False
        out_pdf.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(produced, out_pdf)
    return out_pdf.exists() and out_pdf.stat().st_size > 0


def resolve_truth_source(
    requested: str, arxiv_id: str, no_download: bool, tectonic_ok: bool
) -> str:
    """Decide where the 'truth' PDF comes from: 'arxiv' or 'tectonic'.

    - 'arxiv'/'tectonic' are honored explicitly; an explicit 'tectonic' with
      no tectonic available is an error (never silently switch sources).
    - 'auto' prefers the arXiv canonical PDF when downloads are allowed, else
      renders locally with tectonic if possible, else falls back to the
      (cached) arXiv path.
    """
    if requested == "arxiv":
        return "arxiv"
    if requested == "tectonic":
        if not tectonic_ok:
            raise ValueError(
                "--truth-source=tectonic requested but `tectonic` is not available "
                "(install it or set BYETEX_TECTONIC_BIN)"
            )
        return "tectonic"
    # auto
    if arxiv_id and not no_download:
        return "arxiv"
    if tectonic_ok:
        return "tectonic"
    return "arxiv"


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


# ─────────────────────────────────────────────────────────────────────────────
# PDF structural comparison (content gate before visual review)
# ─────────────────────────────────────────────────────────────────────────────

# Words shorter than this are dropped from the Jaccard set — they're
# typically math-glyph extraction noise (single letters, isolated `e`s
# from `\epsilon` etc.) rather than real prose tokens.
MIN_WORD_LEN = 3

# Cap heading lists at this many entries to keep the JSON small and
# focus heading_recall on the major sections (papers with 50+ subsection
# headings would otherwise drown the signal).
MAX_HEADINGS = 40

_HEADING_NUMBERED_RE = re.compile(r"^\s*(\d+(?:\.\d+)*)\.?\s+(.{2,80})$")
_HEADING_TITLECASE_RE = re.compile(
    r"^([A-Z][a-zA-Z]+)(?:\s+[A-Z][a-zA-Z]+){0,4}\s*$"
)
# Common synonyms: when matching truth headings against typst headings,
# treat these pairs as interchangeable. Typst's `#bibliography(...)`
# emits a "Bibliography" heading even when the LaTeX source said
# `\section{References}`, and similar small renames shouldn't tank
# heading_recall.
_HEADING_SYNONYMS = [
    ("references", "bibliography"),
    ("acknowledgements", "acknowledgments"),
    ("supplementary material", "appendix"),
]
# Unicode ranges + ASCII operators that almost never appear in a real
# section heading. If a candidate line contains any of these, it's
# almost certainly equation text that `pdftotext` lifted out of a
# display-math block — drop it.
_MATH_LIKELY_CHARS = set("·×÷≤≥≠≈≪≫±∓∂∇∫∑∏√∞∈∉⊂⊆⊃⊇∪∩→←↔⇒⇔𝔼𝔽𝕃𝕊ℝℂℕℤℚ𝛼𝛽𝛾𝛿𝜀𝜁𝜂𝜃𝜅𝜆𝜇𝜈𝜉𝜋𝜌𝜎𝜏𝜐𝜑𝜒𝜓𝜔𝛤𝛥𝛩𝛬𝛯𝛱𝛴𝛷𝛹𝛺ƒℎ")


def extract_pdf_text(pdf: Path) -> str:
    """Extract layout-preserving text from a PDF via the `pdftotext` CLI.

    Returns the empty string on error rather than raising — a missing
    or malformed PDF should degrade the structural comparison to "no
    overlap", not crash the whole run.
    """
    try:
        result = subprocess.run(
            ["pdftotext", "-layout", str(pdf), "-"],
            check=True,
            capture_output=True,
            text=True,
        )
        return result.stdout
    except (subprocess.CalledProcessError, FileNotFoundError) as e:
        print(f"  [warn] pdftotext failed on {pdf.name}: {e}", file=sys.stderr)
        return ""


def tokenize_words(text: str) -> set[str]:
    """Return a set of lowercased letter-only tokens, length ≥ MIN_WORD_LEN.

    Math glyphs come through `pdftotext` as a mix of substituted bytes
    and isolated single characters; restricting to ASCII letters of
    reasonable length filters those out so Jaccard reflects prose
    overlap, not equation noise.
    """
    return {
        tok.lower()
        for tok in re.findall(r"[A-Za-z]{%d,}" % MIN_WORD_LEN, text)
    }


# ── Source-anchored truth extraction (D4) ───────────────────────────────────────
# The `pdftotext`-of-PDF heading/float extraction is noisy on math-heavy papers
# (equation fragments masquerade as headings). Since we have the LaTeX source,
# derive the truth heading list and float counts directly from it: ordered,
# clean, and free of equation residue. The source must be the WHOLE project
# (toplevel + every `\input`-ed file) concatenated — corpus papers keep their
# sections in separate files (0 in the toplevel, 7-23 includes).

_SECTION_RE = re.compile(r"\\(?:sub){0,2}section\*?\s*\{", re.IGNORECASE)
_FIGURE_ENV_RE = re.compile(r"\\begin\s*\{\s*figure\*?\s*\}")
_TABLE_ENV_RE = re.compile(r"\\begin\s*\{\s*table\*?\s*\}")


def _strip_comments(tex: str) -> str:
    """Drop LaTeX `%` comments (an unescaped `%` to end of line)."""
    out = []
    for line in tex.splitlines():
        i = 0
        cut = None
        while i < len(line):
            if line[i] == "\\":
                i += 2
                continue
            if line[i] == "%":
                cut = i
                break
            i += 1
        out.append(line if cut is None else line[:cut])
    return "\n".join(out)


def _balanced_group(tex: str, open_idx: int) -> tuple[str, int] | None:
    """Given the index of a `{`, return (inner_text, index_past_close) honoring
    nested braces, or None if unbalanced."""
    depth = 0
    i = open_idx
    while i < len(tex):
        c = tex[i]
        if c == "\\":
            i += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return tex[open_idx + 1 : i], i + 1
        i += 1
    return None


def _clean_heading_title(raw: str) -> str:
    """Normalise a raw `\\section{...}` argument to a comparable title:
    drop `\\label{}`/`\\texorpdfstring`-pdf-arg, strip inline math, remove
    remaining control sequences, collapse whitespace, lowercase."""
    s = raw
    # \texorpdfstring{pdf}{tex} -> keep the PDF (first) arg, which is what renders.
    def _texor(m):
        rest = s[m.end() - 1 :]
        g = _balanced_group(rest, 0)
        return g[0] if g else ""
    while True:
        m = re.search(r"\\texorpdfstring\s*\{", s)
        if not m:
            break
        g1 = _balanced_group(s, m.end() - 1)
        if not g1:
            break
        pdf_arg, after = g1
        # skip the second {tex} arg if present
        tail = s[after:]
        g2_idx = tail.find("{")
        consumed_to = after
        if g2_idx != -1 and tail[:g2_idx].strip() == "":
            g2 = _balanced_group(tail, g2_idx)
            if g2:
                consumed_to = after + g2[1]
        s = s[: m.start()] + pdf_arg + s[consumed_to:]
    # Drop \label{...} entirely.
    while True:
        m = re.search(r"\\label\s*\{", s)
        if not m:
            break
        g = _balanced_group(s, m.end() - 1)
        if not g:
            break
        s = s[: m.start()] + s[g[1] :]
    # Strip inline math `$...$`.
    s = re.sub(r"\$[^$]*\$", " ", s)
    # Remaining control sequences: keep any brace argument's text, drop the cmd.
    s = re.sub(r"\\[a-zA-Z]+\s*", " ", s)
    s = s.replace("{", " ").replace("}", " ")
    return " ".join(s.split()).lower()


def source_headings(tex: str) -> list[str]:
    """Ordered, cleaned `\\section`/`\\subsection`/`\\subsubsection` titles from
    LaTeX source (comments removed). The source-anchored truth heading list."""
    body = _strip_comments(tex)
    out: list[str] = []
    for m in _SECTION_RE.finditer(body):
        g = _balanced_group(body, m.end() - 1)
        if not g:
            continue
        title = _clean_heading_title(g[0])
        if title:
            out.append(title)
        if len(out) >= MAX_HEADINGS:
            break
    return out


def source_float_counts(tex: str) -> dict:
    """Count `figure`/`table` (and starred) environments in LaTeX source
    (comments removed). The source-anchored truth float counts."""
    body = _strip_comments(tex)
    return {
        "figures": len(_FIGURE_ENV_RE.findall(body)),
        "tables": len(_TABLE_ENV_RE.findall(body)),
    }


# Match only levels 1-3 (`=`/`==`/`===`) to mirror `source_headings`, which
# counts `\section`/`\subsection`/`\subsubsection` — NOT `\paragraph` (level 4).
_TYP_HEADING_RE = re.compile(r"^(={1,3})\s+(.+?)\s*$")
_TYP_LABEL_RE = re.compile(r"\s*<[^>]+>\s*$")  # trailing `<label>` anchor
_TYP_HEADING_FN_RE = re.compile(r"^#heading\b[^[]*\[")  # `#heading(..)[`
_TYP_HEADING_LEVEL_RE = re.compile(r"level:\s*(\d+)")  # `#heading(level: 4, …)`


def _unescaped_dollar_count(s: str) -> int:
    """Count `$` not preceded by a backslash (Typst math-mode delimiters)."""
    cnt = 0
    i = 0
    while i < len(s):
        if s[i] == "\\":
            i += 2
            continue
        if s[i] == "$":
            cnt += 1
        i += 1
    return cnt


def _clean_typ_heading(raw: str) -> str:
    """Normalise a raw typst heading title (label stripped) to the comparable
    form: drop inline math, typst markup chars, backslashes; collapse, lower."""
    title = _TYP_LABEL_RE.sub("", raw)
    title = re.sub(r"\$[^$]*\$", " ", title)        # inline math
    title = re.sub(r"[*_`#]", "", title)             # typst markup chars
    title = title.replace("\\", "")
    return " ".join(title.split()).lower()


def typ_float_counts(typ_text: str) -> dict:
    """Count real figures/tables in byetex's generated Typst. byetex wraps many
    NON-float constructs as `#figure(kind: "equation"|"remark"|"theorem"|...)`
    (numbered equations, theorem-like blocks, label anchors), so the PDF-side
    "Figure N"/"Table N" caption count over-reports. Count the `.typ` instead:
    each `#figure(` block is a FIGURE iff its body holds an `image(` call and it
    has no non-image `kind:`; a TABLE iff it carries `kind: table` / `kind:
    "table"`. Equation/anchor/theorem-like kinds are excluded from both."""
    figures = 0
    tables = 0
    # Scan each `#figure(` block: take a window up to the matching close or the
    # next `#figure(`. A line-based heuristic is enough for byetex's output,
    # where `kind:` and `image(`/`table(` appear within a few lines of the open.
    starts = [m.start() for m in re.finditer(r"#figure\(", typ_text)]
    for idx, s in enumerate(starts):
        end = starts[idx + 1] if idx + 1 < len(starts) else len(typ_text)
        block = typ_text[s:end]
        # `kind:` value if present (bare ident or quoted string).
        km = re.search(r'kind:\s*"?([a-zA-Z]+)"?', block)
        kind = km.group(1) if km else None
        if kind == "table":
            tables += 1
        elif kind in (None, "image") and ("image(" in block):
            figures += 1
        # else: equation / anchor / remark / theorem / proposition / … — skip.
    return {"figures": figures, "tables": tables}


def typ_headings(typ_text: str) -> list[str]:
    """Ordered, cleaned headings from byetex's OWN generated Typst. Catches BOTH
    forms byetex emits: line-leading `=`/`==`/… markers (numbered sections) AND
    the `#heading(...)[Title]` function form (starred/unnumbered sections like
    `\\section*{Acknowledgments}` → `#heading(numbering: none)[Acknowledgments]`).
    This anchors the TYPST side the same way `source_headings` anchors the truth
    side, so heading_recall compares clean-vs-clean instead of
    clean-truth-vs-noisy-pdftotext. Strips trailing `<label>`, inline math, and
    `*bold*`/`_emph_` markup; lowercases; collapses whitespace."""
    out: list[str] = []
    dollars_before = 0  # running count of unescaped `$` seen before this line
    for line in typ_text.splitlines():
        line = line.rstrip()
        in_math = dollars_before % 2 == 1
        dollars_before += _unescaped_dollar_count(line)
        # A `=`-leading line INSIDE a multi-line `$ … $` display equation (e.g.
        # `= chevron.l f,g chevron.r`) is the equation's equals sign, NOT a
        # heading. Skip lines that open inside an unclosed math block.
        if in_math:
            continue
        title: str | None = None
        m = _TYP_HEADING_RE.match(line)
        if m:
            title = _clean_typ_heading(m.group(2))
        else:
            fm = _TYP_HEADING_FN_RE.match(line)
            if fm:
                # Skip `\paragraph`/`\subparagraph` (`#heading(level: 4+, …)`) so
                # the typst side matches `source_headings`' level-1-3 scope.
                lvl = _TYP_HEADING_LEVEL_RE.search(line[: fm.end()])
                if lvl and int(lvl.group(1)) > 3:
                    continue
                # Extract the `[...]` content arg, honoring nested brackets.
                start = fm.end() - 1  # index of the opening `[`
                depth = 0
                end = None
                for i in range(start, len(line)):
                    c = line[i]
                    if c == "[":
                        depth += 1
                    elif c == "]":
                        depth -= 1
                        if depth == 0:
                            end = i
                            break
                if end is not None:
                    title = _clean_typ_heading(line[start + 1 : end])
        if title:
            out.append(title)
        if len(out) >= MAX_HEADINGS:
            break
    return out


def extract_pdf_headings(text: str) -> list[str]:
    """Heuristically pull section-heading-like lines out of `pdftotext`
    output.

    Matches two shapes:
      - numbered: `1. Introduction`, `2.1 Setup`, `3.2.1 Lemmas`
      - title-case short line: up to 5 capitalised words, no trailing
        punctuation (catches unnumbered sections like
        `Acknowledgments`)

    Returns lowercased, whitespace-collapsed strings, deduped, in
    document order, capped at MAX_HEADINGS.
    """
    out: list[str] = []
    seen: set[str] = set()
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        # Skip lines that look like equation residue: lines containing
        # the typical math operators or Unicode math glyphs we map to
        # in Typst (`arrow.r`, greek letters, `±`, `∇`, etc.) almost
        # never represent a real heading.
        if any(c in _MATH_LIKELY_CHARS for c in line):
            continue
        # Skip lines that are mostly punctuation/digits — equation
        # snippets often have `+ ( ) − ` with very few letters.
        letters = sum(1 for c in line if c.isalpha())
        if letters < max(3, len(line) // 4):
            continue
        heading: str | None = None
        m = _HEADING_NUMBERED_RE.match(line)
        if m:
            heading = m.group(2).strip()
        elif _HEADING_TITLECASE_RE.match(line) and len(line) <= 60:
            heading = line.strip()
        if not heading:
            continue
        norm = " ".join(heading.lower().split())
        # Strip a few obvious non-headings that match the patterns by
        # accident — figure/table captions, "Page N", running headers.
        if (
            len(norm) < 3
            or norm.startswith(("figure ", "table ", "page ", "equation ", "section "))
            or norm.isdigit()
        ):
            continue
        if norm in seen:
            continue
        seen.add(norm)
        out.append(norm)
        if len(out) >= MAX_HEADINGS:
            break
    return out


def _heading_match(a: str, b: str) -> bool:
    """Two headings match if one substring-contains the other, OR they're
    on a known synonym pair (`references` ↔ `bibliography` and
    friends).
    """
    if a in b or b in a:
        return True
    for x, y in _HEADING_SYNONYMS:
        if (x in a and y in b) or (y in a and x in b):
            return True
    return False


def heading_recall(truth: list[str], typst: list[str]) -> float:
    """Fraction of `truth` headings that have a substring (or synonym)
    match in `typst`'s heading list (either direction — absorbs minor
    renaming).
    """
    if not truth:
        return 1.0  # nothing to recall
    matched = 0
    for h in truth:
        for t in typst:
            if _heading_match(h, t):
                matched += 1
                break
    return matched / len(truth)


# ── Phase-2a structural fidelity metrics ────────────────────────────────────────
# These complement the set-based word_recall / heading_recall, which are blind to
# content *volume* (deletion/duplication), heading *order*, and dropped *floats* —
# the exact structural failures that compile-rate and word/heading recall miss.

def _word_count(text: str) -> int:
    """Number of prose tokens (same tokenization rule as tokenize_words, but
    counting occurrences rather than the unique set)."""
    return len(re.findall(r"[A-Za-z]{%d,}" % MIN_WORD_LEN, text))


def word_recall_set(truth_text: str, typst_text: str) -> float:
    """Set-based word recall: |truth∩typst| / |truth| over UNIQUE tokens. This
    is the existing word_recall computed inline in pdf_structure_compare, pulled
    out so its blind spot (it ignores deletion/duplication of already-present
    words) can be asserted directly against word_count_ratio in tests."""
    t = tokenize_words(truth_text)
    if not t:
        return 0.0
    return len(t & tokenize_words(typst_text)) / len(t)


def word_count_ratio(truth_text: str, typst_text: str):
    """typst prose-token COUNT / truth prose-token count. Catches what the
    set-based recall cannot: a deleted paragraph drops the ratio below 1.0 even
    when its words appear elsewhere, and a duplicated block (e.g. a leaked author
    list) pushes it above 1.0. Returns None when truth has no prose tokens
    (ratio undefined — never divides by zero)."""
    truth_n = _word_count(truth_text)
    if truth_n == 0:
        return None
    return _word_count(typst_text) / truth_n


def heading_sequence_score(truth: list[str], typst: list[str]):
    """Length of the longest IN-ORDER matched subsequence of truth headings
    present in typst, divided by the number of truth headings. Unlike
    heading_recall (a set membership fraction), this penalizes reordered or
    flattened structure: headings must appear in the same relative order to
    count. Matching uses the same _heading_match (substring/synonym) as recall.
    Returns 1.0 when there are no truth headings (nothing to recall), matching
    heading_recall's convention."""
    if not truth:
        return 1.0
    n, m = len(truth), len(typst)
    # Classic LCS over the two heading lists, with _heading_match as equality.
    dp = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(n - 1, -1, -1):
        for j in range(m - 1, -1, -1):
            if _heading_match(truth[i], typst[j]):
                dp[i][j] = 1 + dp[i + 1][j + 1]
            else:
                dp[i][j] = max(dp[i + 1][j], dp[i][j + 1])
    return dp[0][0] / len(truth)


_FIGURE_CAPTION_RE = re.compile(r"\bfigure\s+(\d+)", re.IGNORECASE)
_TABLE_CAPTION_RE = re.compile(r"\btable\s+(\d+)", re.IGNORECASE)


def float_count_ratio(truth_text: str, typst_text: str) -> dict:
    """Ratio of distinct figure/table captions ("Figure N" / "Table N") in the
    typst render vs the truth render. Dropped figures/tables are invisible to
    word and heading metrics but are a real structural regression. Counts
    DISTINCT caption numbers (so repeated in-text references to "Figure 1" don't
    inflate the count). Each ratio is None when truth has none of that float
    type (undefined — never divides by zero)."""
    def distinct(rx, text):
        return {m for m in rx.findall(text)}

    tf = distinct(_FIGURE_CAPTION_RE, truth_text)
    tt = distinct(_TABLE_CAPTION_RE, truth_text)
    yf = distinct(_FIGURE_CAPTION_RE, typst_text)
    yt = distinct(_TABLE_CAPTION_RE, typst_text)
    return {
        "figure_ratio": (len(yf) / len(tf)) if tf else None,
        "table_ratio": (len(yt) / len(tt)) if tt else None,
        "truth_figures": len(tf),
        "truth_tables": len(tt),
        "typst_figures": len(yf),
        "typst_tables": len(yt),
    }


def pdf_structure_compare(
    truth_pdf: Path,
    typst_pdf: Path,
    truth_pages: int,
    typst_pages: int,
    page_min: float,
    page_max: float,
    jaccard_min: float,
    word_recall_min: float,
    heading_recall_min: float,
    source_tex: str | None = None,
    typst_tex: str | None = None,
) -> dict:
    """Compute the structural-similarity dict and gate it against the
    configured thresholds. See the plan doc for metric definitions.

    When `source_tex` (the concatenated project LaTeX) is given, the TRUTH
    headings and TRUTH float counts are taken from the source (D4: noise-free,
    ordered) instead of from `pdftotext` of the rendered PDF, which misfires on
    math-heavy papers. The typst side is always measured from its rendered PDF.
    """
    truth_text = extract_pdf_text(truth_pdf)
    typst_text = extract_pdf_text(typst_pdf)
    truth_words = tokenize_words(truth_text)
    typst_words = tokenize_words(typst_text)
    intersect = truth_words & typst_words
    union = truth_words | typst_words
    word_jaccard = (len(intersect) / len(union)) if union else 0.0
    word_recall = (len(intersect) / len(truth_words)) if truth_words else 0.0

    # Truth headings: source-anchored when available, else pdftotext heuristic.
    truth_from_source = source_tex is not None
    if truth_from_source:
        truth_headings = source_headings(source_tex)
    else:
        truth_headings = extract_pdf_headings(truth_text)
    # Typst side: prefer byetex's OWN `.typ` markers (clean) over pdftotext of
    # its PDF (noisy) — so a source-anchored truth is compared clean-vs-clean.
    if typst_tex is not None:
        typst_headings = typ_headings(typst_tex)
    else:
        typst_headings = extract_pdf_headings(typst_text)
    h_recall = heading_recall(truth_headings, typst_headings)

    # Phase-2a structural metrics (reported; not yet gated — thresholds will be
    # set once the corpus baseline shows realistic cross-engine ranges).
    wc_ratio = word_count_ratio(truth_text, typst_text)
    h_seq = heading_sequence_score(truth_headings, typst_headings)
    floats = float_count_ratio(truth_text, typst_text)
    if truth_from_source:
        # Anchor BOTH sides of the float ratio (like headings): truth counts
        # from the source LaTeX, typst counts from byetex's `.typ` when given.
        # The PDF-side "Figure N"/"Table N" count over-reports because byetex
        # renders equation/theorem/anchor blocks as `#figure(kind: …)`; counting
        # real image figures + `kind: table` from the `.typ` removes that noise.
        src_floats = source_float_counts(source_tex)
        floats = dict(floats)
        floats["truth_figures"] = src_floats["figures"]
        floats["truth_tables"] = src_floats["tables"]
        if typst_tex is not None:
            tf = typ_float_counts(typst_tex)
            floats["typst_figures"] = tf["figures"]
            floats["typst_tables"] = tf["tables"]
        floats["figure_ratio"] = (
            floats["typst_figures"] / src_floats["figures"]
            if src_floats["figures"] else None
        )
        floats["table_ratio"] = (
            floats["typst_tables"] / src_floats["tables"]
            if src_floats["tables"] else None
        )

    page_ratio = (typst_pages / truth_pages) if truth_pages > 0 else 0.0

    fail_reasons: list[str] = []
    if not (page_min <= page_ratio <= page_max):
        fail_reasons.append(
            f"page_ratio {page_ratio:.2f} outside [{page_min:.2f}, {page_max:.2f}]"
        )
    if word_jaccard < jaccard_min:
        fail_reasons.append(f"word_jaccard {word_jaccard:.2f} < {jaccard_min:.2f}")
    if word_recall < word_recall_min:
        fail_reasons.append(f"word_recall {word_recall:.2f} < {word_recall_min:.2f}")
    if h_recall < heading_recall_min:
        fail_reasons.append(
            f"heading_recall {h_recall:.2f} < {heading_recall_min:.2f}"
        )

    return {
        "truth_word_count": len(truth_words),
        "typst_word_count": len(typst_words),
        "word_jaccard": round(word_jaccard, 3),
        "word_recall": round(word_recall, 3),
        "truth_headings": truth_headings,
        "typst_headings": typst_headings,
        "heading_recall": round(h_recall, 3),
        "page_ratio": round(page_ratio, 3),
        # Phase-2a structural metrics (None when undefined — empty truth / no floats).
        "word_count_ratio": round(wc_ratio, 3) if wc_ratio is not None else None,
        "heading_sequence_score": round(h_seq, 3) if h_seq is not None else None,
        "figure_ratio": round(floats["figure_ratio"], 3) if floats["figure_ratio"] is not None else None,
        "table_ratio": round(floats["table_ratio"], 3) if floats["table_ratio"] is not None else None,
        "truth_figures": floats["truth_figures"],
        "typst_figures": floats["typst_figures"],
        "truth_tables": floats["truth_tables"],
        "typst_tables": floats["typst_tables"],
        "structure_ok": not fail_reasons,
        "fail_reasons": fail_reasons,
    }


def page_image_similarity(truth_pages: list[Path], typst_pages: list[Path]) -> dict:
    """Per-page SSIM between rasterized truth and typst pages.

    Compares page i to page i up to the shorter list (page-count drift is
    captured separately by page_ratio). Each pair is greyscaled and resized
    to a common size before SSIM. Cross-engine renders never reach 1.0
    (different fonts/justification/float placement), so use this as a
    *relative* regression detector, not an absolute quality gate.

    Degrades gracefully (mean_ssim=None) when numpy/scikit-image aren't
    installed, so the rest of the pipeline still runs without those deps.
    """
    try:
        import numpy as np
        from skimage.metrics import structural_similarity as ssim
    except ImportError:
        return {
            "mean_ssim": None,
            "per_page_ssim": [],
            "pages_compared": 0,
            "skipped": "install numpy + scikit-image for SSIM",
        }

    n = min(len(truth_pages), len(typst_pages))
    per_page: list[float] = []
    for i in range(n):
        a = Image.open(truth_pages[i]).convert("L")
        b = Image.open(typst_pages[i]).convert("L")
        # Resize both to the smaller common dims to avoid upscaling-blur bias.
        w = min(a.width, b.width)
        h = min(a.height, b.height)
        a = a.resize((w, h))
        b = b.resize((w, h))
        score = float(ssim(np.asarray(a), np.asarray(b)))
        per_page.append(round(score, 3))

    mean = round(sum(per_page) / len(per_page), 3) if per_page else 0.0
    return {"mean_ssim": mean, "per_page_ssim": per_page, "pages_compared": n}


# Weights for the single corpus-wide fidelity number. Prose dominates, with
# structure (headings), visual layout (SSIM) and page density (page_closeness)
# as secondary signals. Tunable as the corpus grows.
FIDELITY_WEIGHTS = {
    "word_recall": 0.35,
    "heading_recall": 0.25,
    "mean_ssim": 0.2,
    "page_closeness": 0.2,
}


def page_closeness(page_ratio: float | None) -> float | None:
    """Map a page_ratio (typst_pages / truth_pages) to a [0, 1] closeness score:
    1.0 when the page count matches exactly, decreasing symmetrically as the
    output runs either LONGER (ratio > 1) or SHORTER (ratio < 1) than the truth.

    `min(r, 1/r)` is the symmetric measure — a 1.25× over-run and a 0.8×
    under-run both score 0.8. Returns None for a missing/degenerate ratio so the
    paper is skipped (mirrors the other metrics' None handling).
    """
    if page_ratio is None or page_ratio <= 0:
        return None
    return min(page_ratio, 1.0 / page_ratio)


def aggregate_fidelity_score(papers: dict) -> float | None:
    """Mean over papers (that carry all metrics) of the weighted blend
    0.35*word_recall + 0.25*heading_recall + 0.2*mean_ssim + 0.2*page_closeness.

    `page_closeness` is derived from each paper's `page_ratio` (see
    `page_closeness`). Papers missing any metric are skipped rather than scored
    as zero, so the number reflects only papers we could actually measure.
    Returns None when no paper has the full set. A *relative* regression
    detector.
    """
    scores: list[float] = []
    for p in papers.values():
        vals = {
            "word_recall": p.get("word_recall"),
            "heading_recall": p.get("heading_recall"),
            "mean_ssim": p.get("mean_ssim"),
            "page_closeness": page_closeness(p.get("page_ratio")),
        }
        if any(v is None for v in vals.values()):
            continue
        scores.append(sum(FIDELITY_WEIGHTS[k] * vals[k] for k in FIDELITY_WEIGHTS))
    if not scores:
        return None
    return round(sum(scores) / len(scores), 3)


# ─────────────────────────────────────────────────────────────────────────────
# Vision-grading packet: high-DPI front-matter crops + a per-paper evidence index
# ─────────────────────────────────────────────────────────────────────────────

def rasterize_page1_highres(pdf: Path, prefix: Path, dpi: int = 200) -> "Path | None":
    """Rasterize only page 1 of a PDF at high DPI; return the single PNG or None."""
    prefix.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["pdftoppm", "-r", str(dpi), "-f", "1", "-l", "1", "-png", str(pdf), str(prefix)],
        check=True,
        capture_output=True,
    )
    pages = sorted(
        prefix.parent.glob(f"{prefix.name}-*.png"),
        key=lambda p: int(re.search(r"-(\d+)\.png$", p.name).group(1)),
    )
    return pages[0] if pages else None


def crop_front_matter(page_png: Path, out: Path, top_frac: float = 0.40) -> Path:
    """Crop the full-width top `top_frac` of a page raster (title/author/abstract band)."""
    out.parent.mkdir(parents=True, exist_ok=True)
    with Image.open(page_png) as img:
        w, h = img.size
        img.crop((0, 0, w, int(h * top_frac))).save(out)
    return out


# Map known base documentclasses to friendly labels.
_DOC_CLASS_MAP = {
    "IEEEtran": "ieeetran",
    "acmart": "acmart",
    "llncs": "lncs",
    "elsarticle": "elsarticle",
    "article": "article",
    "report": "article",
}


def detect_doc_class(toplevel_tex: Path) -> "str | None":
    """Detect the (friendly) document class from a top-level .tex; None if unknown."""
    try:
        text = toplevel_tex.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    m = re.search(r"\\documentclass(\[[^]]*\])?\{([a-zA-Z]+)\}", text)
    if not m:
        return None
    base = m.group(2)
    # article/report with a conference style package → conference label. The
    # package may carry a path prefix, e.g. \usepackage{style/neurips_2026}, so
    # match the family name ANYWHERE inside the \usepackage{...} braces.
    if base in ("article", "report"):
        for pkg in re.findall(r"\\usepackage(?:\[[^]]*\])?\{([^}]*)\}", text):
            for label in ("neurips", "icml", "iclr"):
                if label in pkg.lower():
                    return label
    return _DOC_CLASS_MAP.get(base)


def build_grading_packet(
    out_dir: Path,
    summary: dict,
    front_matter: dict,
    detected_class: "str | None",
    rubric_rel: str = "docs/fidelity-rubric.md",
) -> Path:
    """Write out_dir/grading_packet.json: a portable index of all grading evidence."""
    pages_dir = out_dir / "pages"

    def _by_num(prefix: str) -> dict:
        # Pair the standard per-page rasters (`<side>-NN.png`). Exclude the
        # high-DPI front-matter intermediates (`<side>-fm-NN.png`), which share
        # the `<side>-*` glob and would otherwise collide on page 1.
        found: dict[int, str] = {}
        if pages_dir.is_dir():
            for p in pages_dir.glob(f"{prefix}-*.png"):
                if f"{prefix}-fm-" in p.name:
                    continue
                mm = re.search(r"-(\d+)\.png$", p.name)
                if mm:
                    found[int(mm.group(1))] = f"pages/{p.name}"
        return found

    truth_by_num = _by_num("truth")
    typst_by_num = _by_num("typst")
    pages = []
    for num in sorted(set(truth_by_num) | set(typst_by_num)):
        pages.append({
            "page": num,
            "truth": truth_by_num.get(num),
            "typst": typst_by_num.get(num),
        })

    packet = {
        "id": summary.get("id") or out_dir.name,
        "detected_class": detected_class,
        "truth_source": summary.get("truth_source"),
        "front_matter": front_matter or None,
        "pages": pages,
        "composite": "composite.png",
        "structure": summary.get("structure") or {},
        "warnings": summary.get("warnings") or {},
        "rubric": rubric_rel,
    }
    out = out_dir / "grading_packet.json"
    out.write_text(json.dumps(packet, indent=2) + "\n")
    return out


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
    draw.text((PADDING * 2 + cell_w, 6), "TYPST  byetex", fill=(140, 0, 0))
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
    byetex_bin: Path,
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
        print(f"  [error] No source dir found under corpus/{arxiv_id}/ — run corpus_harvest.py --pinned", file=sys.stderr)
        return summary

    toplevel = find_toplevel_tex(source_dir)
    if toplevel is None:
        summary["status"] = "no_toplevel"
        print(f"  [error] Cannot identify entry .tex in {source_dir}", file=sys.stderr)
        return summary

    summary["toplevel_tex"] = toplevel.name
    print(f"  toplevel: {toplevel.name}", flush=True)

    # 2. Acquire truth PDF — arXiv canonical download or local tectonic render
    # (see --truth-source). Bundled source PDFs are unreliable, so we never
    # use those.
    truth_dest = out_dir / "truth.pdf"
    tectonic_ok = tectonic_available()
    try:
        truth_source = resolve_truth_source(
            args.truth_source, arxiv_id, args.no_truth_download, tectonic_ok
        )
    except ValueError as e:
        summary["status"] = "truth_source_unavailable"
        print(f"  [error] {e}", file=sys.stderr)
        return summary

    if truth_source == "tectonic":
        print(f"  rendering truth PDF with tectonic ...", flush=True)
        if not render_reference_tectonic(toplevel, truth_dest):
            summary["status"] = "truth_render_failed"
            return summary
        summary["truth_source"] = "tectonic"
        print(f"  truth PDF: rendered locally ({truth_dest.stat().st_size // 1024} KB)", flush=True)
    elif truth_dest.exists() and truth_dest.stat().st_size > 10_000:
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

    # 3. Convert with byetex
    print(f"  byetex convert ...", flush=True)
    typ_path, warn_path = run_byetex(byetex_bin, source_dir, toplevel)
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

    # 5b. PDF structural comparison — gate the visual review on
    # whether the truth and typst PDFs actually share their main
    # content. A passing typst compile + similar page count is no
    # longer enough; we need the body text to overlap with the truth.
    if not args.no_structure_check:
        structure = pdf_structure_compare(
            truth_dest,
            typst_pdf,
            len(truth_pages),
            len(typst_pages),
            page_min=args.min_page_ratio,
            page_max=args.max_page_ratio,
            jaccard_min=args.min_word_jaccard,
            word_recall_min=args.min_word_recall,
            heading_recall_min=args.min_heading_recall,
            # Source-anchored truth headings/float counts (D4): noise-free,
            # ordered, from the project LaTeX rather than pdftotext of the PDF.
            source_tex=collect_project_source(toplevel),
            # Byetex's own .typ heading markers (clean) for the typst side.
            typst_tex=typ_path.read_text(encoding="utf-8", errors="replace"),
        )

        # Per-page SSIM (visual/layout fidelity). Recorded always; warning-only
        # — never part of the hard gate, since cross-engine SSIM never hits 1.0.
        ssim_res = page_image_similarity(truth_pages, typst_pages)
        structure["mean_ssim"] = ssim_res["mean_ssim"]
        structure["per_page_ssim"] = ssim_res["per_page_ssim"]
        structure["ssim_pages_compared"] = ssim_res["pages_compared"]
        mean_ssim = ssim_res["mean_ssim"]
        if (
            args.min_mean_ssim > 0
            and mean_ssim is not None
            and mean_ssim < args.min_mean_ssim
        ):
            structure["ssim_below_threshold"] = True
            print(
                f"  [warn] mean_ssim {mean_ssim:.2f} < {args.min_mean_ssim:.2f} "
                f"(warning only — not a gate)",
                flush=True,
            )

        summary["structure"] = structure
        (out_dir / "structure.json").write_text(
            json.dumps(structure, indent=2) + "\n"
        )
        verdict = "ok" if structure["structure_ok"] else "FAIL"
        ssim_str = f"{mean_ssim:.2f}" if mean_ssim is not None else "n/a"
        print(
            f"  structure: jaccard={structure['word_jaccard']:.2f} "
            f"recall={structure['word_recall']:.2f} "
            f"headings={structure['heading_recall']:.2f} "
            f"ssim={ssim_str} "
            f"page_ratio={structure['page_ratio']:.2f} → {verdict}",
            flush=True,
        )
        if not structure["structure_ok"]:
            for reason in structure["fail_reasons"]:
                print(f"    fail: {reason}", flush=True)
            summary["status"] = "structure_failed"
            # Fall through to build the composite — it's still useful
            # for diagnosing *why* the structure didn't match.

    # 5c. Optional self-check: render the source with tectonic and compare to
    # the arXiv truth. A low overlap means the source doesn't reproduce its own
    # arXiv PDF — i.e. the source itself is suspect, not necessarily ByeTex.
    if args.tectonic_crosscheck and summary["truth_source"] in ("arxiv_download", "cached") and tectonic_ok:
        print(f"  tectonic self-check ...", flush=True)
        ref_pdf = out_dir / "tectonic_ref.pdf"
        if render_reference_tectonic(toplevel, ref_pdf):
            tw = tokenize_words(extract_pdf_text(truth_dest))
            xw = tokenize_words(extract_pdf_text(ref_pdf))
            uni = tw | xw
            jac = (len(tw & xw) / len(uni)) if uni else 0.0
            summary["truth_selfcheck"] = {"word_jaccard": round(jac, 3), "reproduces": jac >= 0.5}
            print(f"  truth self-check (arXiv vs tectonic): jaccard={jac:.2f}", flush=True)

    # 6. Build composite
    print(f"  building composite ...", flush=True)
    build_composite(truth_pages, typst_pages, composite_path, arxiv_id)
    kb = composite_path.stat().st_size // 1024
    print(f"  composite.png: {kb} KB", flush=True)

    # 6b. Vision-grading packet: high-DPI page-1 front-matter crops + an index
    # pointing at every piece of evidence with the metrics inlined. A crop
    # failure must not abort the paper — the full page rasters remain.
    fm: dict = {}
    try:
        for side, pdf in (("truth", truth_dest), ("typst", typst_pdf)):
            p1 = rasterize_page1_highres(
                pdf, pages_dir / f"{side}-fm", args.front_matter_dpi
            )
            if p1 is not None:
                crop_front_matter(p1, pages_dir / f"frontmatter-{side}.png")
                fm[side] = f"pages/frontmatter-{side}.png"
                # Drop the full-page high-DPI intermediate; only the crop is kept
                # (and it would otherwise clutter the per-page raster set).
                p1.unlink(missing_ok=True)
    except Exception as e:  # noqa: BLE001 — best-effort; never abort the paper
        print(f"  [warn] front-matter crop failed: {e}", flush=True)
        fm = {}
    try:
        cls = detect_doc_class(toplevel)
        summary["detected_class"] = cls
        build_grading_packet(out_dir, {**summary, "id": arxiv_id}, fm, cls)
        print(f"  grading_packet.json + front-matter crops", flush=True)
    except Exception as e:  # noqa: BLE001
        print(f"  [warn] grading packet failed: {e}", flush=True)

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
        help="arXiv IDs to process (default: pinned set from corpus/manifest.json)",
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
        help="build byetex in release mode (default)",
    )
    p.add_argument(
        "--debug", dest="profile", action="store_const", const="debug",
        help="build byetex in debug mode (faster build, slower binary)",
    )
    p.add_argument(
        "--rasterize-dpi", type=int, default=RASTERIZE_DPI, metavar="DPI",
        help=f"pdftoppm DPI for rasterization (default: {RASTERIZE_DPI})",
    )
    p.add_argument(
        "--front-matter-dpi", type=int, default=200, metavar="DPI",
        help="pdftoppm DPI for the high-res page-1 front-matter crop (default: 200)",
    )
    p.add_argument(
        "--no-truth-download", action="store_true",
        help="error if truth PDF is not already on disk",
    )
    p.add_argument(
        "--truth-source", choices=["arxiv", "tectonic", "auto"], default="auto",
        help="source of the 'truth' PDF: arXiv canonical download, local "
             "tectonic render, or auto (arXiv when downloadable, else tectonic). "
             "Default: auto",
    )
    p.add_argument(
        "--tectonic-crosscheck", action="store_true",
        help="even when arXiv is the truth, also render with tectonic and record "
             "how well the source reproduces its own arXiv PDF (truth_selfcheck)",
    )
    p.add_argument(
        "--delay", type=float, default=ARXIV_MIN_DELAY, metavar="SEC",
        help=f"polite delay between arXiv PDF downloads (default: {ARXIV_MIN_DELAY}s)",
    )
    p.add_argument("--user-agent", default=DEFAULT_UA, metavar="UA")
    # Structural-comparison gate (runs between rasterize and composite)
    p.add_argument(
        "--no-structure-check", action="store_true",
        help="skip the PDF source-data structural comparison",
    )
    p.add_argument(
        "--min-page-ratio", type=float, default=0.70, metavar="R",
        help="reject when typst_pages / truth_pages is below this (default: 0.70)",
    )
    p.add_argument(
        "--max-page-ratio", type=float, default=1.30, metavar="R",
        help="reject when typst_pages / truth_pages is above this (default: 1.30)",
    )
    p.add_argument(
        "--min-word-jaccard", type=float, default=0.55, metavar="X",
        help="reject when |T ∩ Y| / |T ∪ Y| below this (default: 0.55)",
    )
    p.add_argument(
        "--min-word-recall", type=float, default=0.65, metavar="X",
        help="reject when |T ∩ Y| / |T| below this (default: 0.65)",
    )
    p.add_argument(
        "--min-heading-recall", type=float, default=0.60, metavar="X",
        help="reject when fraction of truth headings substring-matched in typst's headings is below this (default: 0.60)",
    )
    p.add_argument(
        "--min-mean-ssim", type=float, default=0.0, metavar="X",
        help="warning-only: print a notice when a paper's mean per-page SSIM "
             "is below this (default: 0.0 = off). SSIM is never a hard gate — "
             "cross-engine renders never reach 1.0.",
    )
    args = p.parse_args()

    out = args.out if args.out.is_absolute() else (REPO_ROOT / args.out)
    out.mkdir(parents=True, exist_ok=True)
    index_path = out / "index.json"
    index = load_index(index_path)

    session = make_session(args.user_agent)
    byetex_bin = ensure_byetex(args.profile)

    for arxiv_id in args.papers:
        print(f"\n=== {arxiv_id} ===", flush=True)
        try:
            summary = process_paper(arxiv_id, out, session, byetex_bin, args)
        except Exception as exc:
            import traceback
            print(f"  [fatal] {exc}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
            summary = {"id": f"arxiv:{arxiv_id}", "status": "exception", "error": str(exc)}

        structure = summary.get("structure") or {}
        index["papers"][arxiv_id] = {
            "status": summary.get("status", "unknown"),
            "convert_ok": summary.get("convert_ok", False),
            "typst_ok": summary.get("typst_ok", False),
            "structure_ok": structure.get("structure_ok", False),
            "truth_pages": summary.get("truth_pages", 0),
            "typst_pages": summary.get("typst_pages", 0),
            "page_count_diff": summary.get("page_count_diff"),
            "page_ratio": structure.get("page_ratio"),
            "warnings_total": summary.get("warnings", {}).get("total", 0),
            "word_jaccard": structure.get("word_jaccard"),
            "word_recall": structure.get("word_recall"),
            "heading_recall": structure.get("heading_recall"),
            # Phase-2a structural metrics.
            "word_count_ratio": structure.get("word_count_ratio"),
            "heading_sequence_score": structure.get("heading_sequence_score"),
            "figure_ratio": structure.get("figure_ratio"),
            "table_ratio": structure.get("table_ratio"),
            "truth_figures": structure.get("truth_figures"),
            "typst_figures": structure.get("typst_figures"),
            "truth_tables": structure.get("truth_tables"),
            "typst_tables": structure.get("typst_tables"),
            "mean_ssim": structure.get("mean_ssim"),
            "composite": str(out / arxiv_id.replace("/", "_") / "composite.png")
                if summary.get("typst_ok") else None,
        }
        flush_index(index, index_path)
        print(f"  → status: {summary.get('status')}", flush=True)

    ok_count = sum(1 for v in index["papers"].values() if v["status"] == "ok")
    structure_ok_count = sum(
        1 for v in index["papers"].values() if v.get("structure_ok")
    )
    typst_ok_count = sum(
        1 for v in index["papers"].values() if v.get("typst_ok")
    )
    # Single corpus-wide fidelity number (relative regression detector).
    fidelity = aggregate_fidelity_score(index["papers"])
    index["fidelity_score"] = fidelity
    flush_index(index, index_path)

    print(f"\nDone: {ok_count}/{len(args.papers)} fully processed.")
    print(
        f"  Stage counts: typst_ok={typst_ok_count} | "
        f"structure_ok={structure_ok_count} | overall_ok={ok_count}"
    )
    fidelity_str = f"{fidelity:.3f}" if fidelity is not None else "n/a (no fully-measured papers)"
    print(
        "  Corpus fidelity score "
        f"(0.35·recall + 0.25·headings + 0.2·ssim + 0.2·page): {fidelity_str}"
    )
    print(f"Index: {index_path}")
    print("Next: ask the agent to read each composite.png and write tests/visual/report.md")


if __name__ == "__main__":
    main()
