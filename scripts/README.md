# scripts/

Dev-only helper scripts for the ByeTex project. Not part of the Rust build.

## harvest_templates.py

Downloads LaTeX templates from [latextemplates.com](https://www.latextemplates.com/) and
arXiv source tarballs into the local `templates/` directory (alongside the curated,
committed templates) for testing the LaTeX→Typst converter. The harvested
subdirectories (`templates/latextemplates/`, `templates/arxiv/`, plus
`templates/manifest.json`) are gitignored — run the script to populate them
locally.

### Setup

**With [uv](https://github.com/astral-sh/uv) (recommended — no venv needed):**

```bash
uv run --with requests --with beautifulsoup4 python scripts/harvest_templates.py --dry-run
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
uv run --with requests --with beautifulsoup4 python scripts/harvest_templates.py --dry-run
```

**Small batch** (5 items, mixed sources — verify this works before going large):

```bash
uv run --with requests --with beautifulsoup4 python scripts/harvest_templates.py --limit 5
```

**Resume a partial run:**

```bash
uv run --with requests --with beautifulsoup4 python scripts/harvest_templates.py --limit 5 --resume
```

**Larger batch** (confirm before running):

```bash
uv run --with requests --with beautifulsoup4 python scripts/harvest_templates.py \
    --source latextemplates --no-limit
uv run --with requests --with beautifulsoup4 python scripts/harvest_templates.py \
    --source arxiv --limit 30 \
    --arxiv-category cs.LG --arxiv-category math.NA --arxiv-category stat.ML
```

**arXiv only, custom categories:**

```bash
uv run --with requests --with beautifulsoup4 python scripts/harvest_templates.py \
    --source arxiv --limit 10 --arxiv-category math.AG
```

### Output layout

```
templates/
  README.md                            # committed — layout doc
  IEEE/  ACM/  NeurIPS/  thesis/       # committed — curated hand-written templates
  manifest.json                        # harvested (gitignored)
  latextemplates/                      # harvested (gitignored)
    essay/
      tufte-essay/
        source.zip                     # original archive
        source/                        # extracted files
        meta.json                      # per-item metadata
    academic-paper/
      ...
  arxiv/                               # harvested (gitignored)
    cs_LG/
      2406.12345/
        source.tar.gz
        source/
        meta.json
```

### License note

Templates from latextemplates.com are often licensed under **CC BY-NC-SA** (non-commercial).
They are downloaded here solely for research/testing purposes. Check `manifest.json`
for the per-item license field before any other use.

arXiv source downloads are governed by each paper's stated license (see `license_url`
in `manifest.json`). Many use the arXiv non-exclusive distribution license, which
permits this kind of research use.
