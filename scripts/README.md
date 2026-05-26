# scripts/

Dev-only helper scripts for the ByeTex project. Not part of the Rust build.

---

## corpus_harvest.py

Downloads arXiv source tarballs into `corpus/<id>/source/` for testing the
LaTeX→Typst converter against real-world papers. Driven by `corpus/manifest.json`
(committed); payloads are gitignored.

### Setup

**With [uv](https://github.com/astral-sh/uv) (recommended — no venv needed):**

```bash
uv run --with requests python scripts/corpus_harvest.py --pinned
```

**With pip:**

```bash
pip install requests
python scripts/corpus_harvest.py --pinned
```

### Usage

**Fetch the 5 pinned regression papers** (used by CI + `template_budgets` tests):

```bash
uv run --with requests python scripts/corpus_harvest.py --pinned
```

**Fetch all papers in the manifest** (26 total):

```bash
uv run --with requests python scripts/corpus_harvest.py
```

**Dry run** (no downloads — shows what would be fetched):

```bash
uv run --with requests python scripts/corpus_harvest.py --dry-run
uv run --with requests python scripts/corpus_harvest.py --pinned --dry-run
```

**Add new papers from arXiv and fetch them:**

```bash
uv run --with requests python scripts/corpus_harvest.py --search cs.LG --limit 5
```

### Output layout

```
corpus/
  manifest.json          # committed — source of truth
  2605.22507/            # gitignored payload
    source.tar.gz
    source/              # extracted LaTeX files
  2605.22557/
    ...
```

### License note

arXiv source downloads are governed by each paper's stated license (see `license_url`
in `corpus/manifest.json`). Many use the arXiv non-exclusive distribution license,
which permits this kind of research use.

---

## visual_test.py

Runs the visual regression pipeline: for each arXiv paper in `corpus/`,
runs `byetex convert` → `typst compile` → rasterizes both PDFs → builds a
side-by-side composite PNG for agent visual grading.

The default paper set is the 5 pinned IDs from `corpus/manifest.json`.
Run `corpus_harvest.py --pinned` first to ensure the payloads are present.

```bash
uv run --with requests --with Pillow python scripts/visual_test.py
uv run --with requests --with Pillow python scripts/visual_test.py --papers 2605.22507
uv run --with requests --with Pillow python scripts/visual_test.py --skip-existing
```

Output goes into `tests/visual/` (gitignored). After the script finishes, read
each `composite.png` and write findings to `tests/visual/report.md`.

---

## corpus_sweep.sh

Fast PASS/FAIL sweep over all fetched arXiv papers. Converts each entry `.tex`
via the `byetex` CLI, compiles with `typst`, and reports totals.

```bash
./scripts/corpus_sweep.sh                 # full sweep
./scripts/corpus_sweep.sh 2605.22507      # single paper
./scripts/corpus_sweep.sh --summary       # totals only
```

---

## render_corpus_summary.py

Updates the warning-count table in `README.md` from `tests/corpus/report.json`.
Called automatically by CI. Not usually run by hand.
