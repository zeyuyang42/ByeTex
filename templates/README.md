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

- `IEEE/` — IEEE conference paper template (`conference_101719.tex` +
  `IEEEtran.cls` + `fig1.png`).

## Adding a new template

1. `mkdir templates/<name>` and drop the LaTeX sources and assets inside.
2. Confirm the LaTeX compiles standalone with `pdflatex` (sanity check that
   the upstream template is intact).
3. Run `bytetex convert templates/<name>/<entry>.tex` and inspect the
   warnings. If specific patterns recur and could plausibly be supported
   deterministically, file an issue or add an emitter rule.
