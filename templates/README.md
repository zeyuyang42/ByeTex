# Templates

End-to-end conversion inputs. Each subdirectory holds the original `.tex`
sources plus any asset files (figures, class files, `.bib`). Generated
outputs (`.typ`, `.warnings.json`, `.pdf`) are gitignored — regenerate them
with:

```bash
bytetex convert templates/<name>/<entry>.tex
typst compile templates/<name>/<entry>.typ
```

## Two flavors live here

**Curated (committed):** hand-written templates exercising specific patterns
we want under regression-test coverage. Small, deliberate, license-clean.

- `IEEE/` — IEEE conference paper (`conference_101719.tex` + `IEEEtran.cls` +
  `fig1.png`). The full upstream IEEE template, untouched.
- `ACM/` — Representative ACM SIGCONF paper (`sample-sigconf.tex`). Exercises
  `acmart`-style title block, author/affiliation/email blocks, `booktabs`
  tables, and a `bibliography` section. The actual `acmart.cls` is not bundled;
  to `pdflatex` this template, fetch acmart from CTAN.
- `NeurIPS/` — Representative NeurIPS-style paper (`neurips_paper.tex`).
  Math-heavy body, `\thanks`-decorated author block, natbib-style citations
  (`\citet`/`\citep`), `align` env. Compile with `neurips_2024.sty` from
  NeurIPS.cc.
- `thesis/` — Self-contained report-class thesis skeleton
  (`thesis_skeleton.tex`). Exercises `\chapter` structure, `amsthm` theorem
  envs, ToC, and list of figures. Requires no external class file; should
  compile cleanly with any LaTeX distribution.

**Harvested (gitignored):** real-world templates downloaded by
`scripts/harvest_templates.py` from latextemplates.com and arXiv. These give
breadth-of-coverage signal for the corpus pass-rate.

- `latextemplates/{essay,academic-paper,...}/<slug>/source/` — extracted
  templates from latextemplates.com.
- `arxiv/<category>/<arxiv-id>/source/` — recent arXiv submission sources.
- `manifest.json` — metadata index (license, title, sha256, fetched_at).

Populate them with:

```bash
uv run --with requests --with beautifulsoup4 \
    python scripts/harvest_templates.py --limit 5
```

(see `scripts/README.md` for the full options).

## Adding a new curated template

1. `mkdir templates/<name>` and drop the LaTeX sources and assets inside.
2. Confirm the LaTeX compiles standalone with `pdflatex` (sanity check that
   the upstream template is intact).
3. Run `bytetex convert templates/<name>/<entry>.tex` and inspect the
   warnings. Add a budget entry in
   `crates/bytetex-core/tests/template_budgets.rs` to lock the regression
   floor. If specific patterns recur, generalize the fix into the emitter.
