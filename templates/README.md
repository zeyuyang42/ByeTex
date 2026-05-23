# Templates

Real-world LaTeX templates used as end-to-end conversion examples. Each
subdirectory holds the original `.tex` sources and any asset files (figures,
class files, `.bib`). Generated outputs (`.typ`, `.warnings.json`, `.pdf`) are
gitignored — regenerate them with:

```bash
bytetex convert templates/<name>/<entry>.tex
typst compile templates/<name>/<entry>.typ
```

## Available templates

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

## Adding a new template

1. `mkdir templates/<name>` and drop the LaTeX sources and assets inside.
2. Confirm the LaTeX compiles standalone with `pdflatex` (sanity check that
   the upstream template is intact).
3. Run `bytetex convert templates/<name>/<entry>.tex` and inspect the
   warnings. If specific patterns recur and could plausibly be supported
   deterministically, file an issue or add an emitter rule.
