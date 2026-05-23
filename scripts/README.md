# scripts/

Dev-only helper scripts for the ByeTex project. Not part of the Rust build.

---

## setup_corpus.py

Populates `corpus/inhouse/` from the committed `tests/inhouse/` source-of-truth.
Run this once before using the inhouse templates with bytetex + typst (so that
generated outputs go into the gitignored `corpus/` rather than `tests/`).

```bash
python scripts/setup_corpus.py
```

Idempotent — re-running refreshes copies without destroying anything.

---

## harvest_templates.py

Downloads arXiv source tarballs into `corpus/online/arxiv/<id>/source/` for
testing the LaTeX→Typst converter against real-world papers. All output is
gitignored — run the script to populate locally.

### Setup

**With [uv](https://github.com/astral-sh/uv) (recommended — no venv needed):**

```bash
uv run --with requests python scripts/harvest_templates.py --dry-run
```

**With a virtual environment:**

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install -r scripts/requirements.txt
python scripts/harvest_templates.py --dry-run
```

### Usage

**Dry run** (no downloads — shows what would be fetched):

```bash
uv run --with requests python scripts/harvest_templates.py --dry-run
```

**Small batch** (5 items — verify before going large):

```bash
uv run --with requests python scripts/harvest_templates.py --limit 5
```

**Resume a partial run:**

```bash
uv run --with requests python scripts/harvest_templates.py --limit 5 --resume
```

**Larger batch with custom categories:**

```bash
uv run --with requests python scripts/harvest_templates.py \
    --no-limit \
    --arxiv-category cs.LG --arxiv-category math.NA --arxiv-category stat.ML
```

### Output layout

```
corpus/
  manifest.json                        # arXiv metadata index (gitignored)
  inhouse/                             # copied from tests/inhouse/ by setup_corpus.py
    ieee/  acm/  neurips/  thesis/
  online/
    arxiv/                             # downloaded by harvest_templates.py
      2605.22507/
        source.tar.gz
        source/                        # extracted LaTeX files
        meta.json                      # per-paper metadata
      2605.22557/
        ...
```

### License note

arXiv source downloads are governed by each paper's stated license (see `license_url`
in `corpus/manifest.json`). Many use the arXiv non-exclusive distribution license,
which permits this kind of research use.

---

## visual_test.py

Runs the visual regression pipeline: for each arXiv paper in `corpus/online/arxiv/`,
runs `bytetex convert` → `typst compile` → rasterizes both PDFs → builds a
side-by-side composite PNG for agent visual grading.

```bash
uv run --with requests --with Pillow python scripts/visual_test.py
uv run --with requests --with Pillow python scripts/visual_test.py --papers 2605.22507
uv run --with requests --with Pillow python scripts/visual_test.py --skip-existing
```

Output goes into `tests/visual/` (gitignored). After the script finishes, read
each `composite.png` and write findings to `tests/visual/report.md`.
