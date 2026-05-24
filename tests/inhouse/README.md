# Inhouse Templates

Committed source-of-truth for the four hand-written regression templates.
Each covers a distinct document class and pattern set; warning budgets are
locked in `crates/bytetex-core/tests/template_budgets.rs`.

To use as conversion inputs, run `python scripts/setup_corpus.py` first —
it copies this tree into `corpus/inhouse/` where the bytetex + typst pipeline
can write generated outputs alongside the source.

## Templates

**`ieee/conference_101719.tex`** — IEEE conference paper.  
Full upstream IEEE template using `IEEEtran.cls`. Exercises IEEE author blocks
(`\IEEEauthorblockN`, `\IEEEauthorblockA`), `\IEEEkeywords`, two-column layout,
`\IEEEpubid`. Budget: 16 warnings (IEEE-class-specific commands covered by a
future IEEE skill).

**`acm/sample-sigconf.tex`** — ACM SIGCONF paper.  
Representative `acmart`-style title block, author/affiliation/email blocks,
`booktabs` tables, and a `bibliography` section. Budget: 0 warnings.

**`neurips/neurips_paper.tex`** — NeurIPS-style paper.  
Math-heavy body, `\thanks`-decorated author block, natbib-style citations
(`\citet`/`\citep`), `align` environment. Budget: 1 warning.

**`thesis/thesis_skeleton.tex`** — Report-class thesis skeleton.  
Exercises `\chapter` structure, `amsthm` theorem environments, ToC, and list
of figures. Requires no external class file. Budget: 0 warnings.
