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

## corpus_add_local.py

Ingest a **non-arXiv** LaTeX project (book/report/thesis/generic article) into the
corpus in the same layout the sweep + visual harness consume — companion to
`corpus_harvest.py`. It copies/extracts the source into `corpus/<id>/source/`,
auto-detects the toplevel `.tex` (the file with both `\documentclass` and
`\begin{document}`), writes `00README.json`, and appends a `manifest.json` entry
recording `source`/`doc_class`/`doc_type` and a re-fetch ref (`repo_ref` commit SHA
or `archive_sha256`).

IDs use a non-arXiv scheme so they never collide with `NNNN.NNNNN`:
`gh-<org>-<repo>`, `ctan-<name>`, `overleaf-<slug>`, `local-<slug>` (lowercase).

```bash
# git repo (records the resolved commit SHA)
python scripts/corpus_add_local.py --git https://github.com/org/repo \
  --id gh-org-repo --source-kind github --doc-type book --title "The Title"

# a hand-downloaded archive (e.g. a login-walled Overleaf export)
python scripts/corpus_add_local.py ~/Downloads/x.zip --id overleaf-x \
  --source-kind overleaf --doc-type thesis --needs-manual-download

# a CTAN / release archive by URL (lazily needs `requests`)
uv run --with requests python scripts/corpus_add_local.py \
  --url https://mirrors.ctan.org/macros/latex/contrib/memoir.zip \
  --id ctan-memoir --source-kind ctan --doc-type book --toplevel doc-src/memman.tex
```

Pass `--toplevel NAME.tex` when 0 or >1 candidates are found, `--dry-run` to preview.
New non-arXiv ids stay **out of `acceptance_baseline.json`** (measurement-only) until
their converter gaps are triaged. For non-arXiv ids, `visual_test.py` auto-builds the
ground-truth PDF with tectonic (no arXiv PDF). See `docs/tier1-baseline-2026-06-15.md`
for the first baseline + ranked gaps.

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
