# Tier-1 (non-academic) baseline — 2026-06-15

First measurement of ByeTex on **long-form prose documents beyond arXiv papers** (books, theses,
reports). 8 diverse real-world projects were ingested via the new `scripts/corpus_add_local.py`
(GitHub clones + one CTAN archive) and run through the existing compile/structure harness. This is
the **measure-first** step of the "beyond academic papers" expansion — it drives the converter work
that follows, replacing guesses with evidence.

## Corpus (8 projects, deliberately diverse)

| id | source class | doc type | compile | note |
|----|----|----|----|----|
| gh-amberj-latex-book-template | `book` | book | ✅ PASS | content faithful, layout compressed |
| gh-fmarotta-kaobook | `kaobook` | book | ✅ PASS | minimal_book example |
| gh-calpolycsc-thesis | `cpthesis` | thesis | ✅ PASS | chapter→L1/section→L2/subsec→L3 OK |
| gh-pelegs-maths-book | `book` | book | ⚠️ "PASS" | **compiles but body is empty** (see #2) |
| gh-dzwaneveld-tudelft-thesis | `tudelft-report` | thesis | ❌ FAIL | unclosed delimiter (#1) |
| gh-maurovm-thesis-template | `oxengthesis` | thesis | ❌ FAIL | unclosed delimiter (#1) |
| ctan-memoir | `memoir` | book (manual) | ❌ FAIL | unclosed delimiter (#1) + counter glue (#3) |
| gh-sikatikenmogne-report | `internshipreport` | report | ❌ FAIL | expected comma (#3) |

**Compile baseline: 4/8 typst-compile; effectively 3/8 usable** (pelegs compiles to an empty body).
All custom-class projects (`tudelft-report`/`oxengthesis`/`internshipreport`/`memoir`/`kaobook`)
correctly fall through to the neutral-preamble path — the failures below are *content* bugs, not
class-detection bugs.

> **Oracle caveat (confirmed):** `corpus_sweep --with-oracle` labels all 4 failures `INPUT_BROKEN`,
> but that is a **truth-build limitation, not exoneration**: tectonic can't build these originals
> because they need fonts ("Roboto Slab Light"), shell-escape (`svg`→pdf), or multi-pass builds
> (`memoir` wants `trims-example.pdf` first) — only amberj built cleanly. The malformed `.typ`
> below are genuine ByeTex output bugs regardless of whether the source builds. See "Measurement
> infrastructure" at the end.

---

## Ranked converter gaps (evidence-backed)

### 1. [COMPILE BLOCKER] Word-boundary emphasis breaks `*…*` / `_…_` — 3 papers
Typst `*`/`_` shorthands delimit emphasis **only at a word boundary**. ByeTex still emits the
shorthand in body text, so it produces an unclosed delimiter whenever emphasis is glued to a
non-word char:
- `ctan-memoir:3626` — `yesteryear?*Franois Villon*` (`*` glued after `?`)
- `gh-maurovm-thesis-template:510` — `` ``_…viewpoints…_'' `` (`_` glued after a backtick quote)
- `gh-dzwaneveld-tudelft-thesis:118` — `_An introduction… @example-article_` (`_` against a `@ref`)

This is the **same bug class** as `project-typst-word-boundary` / PR #226, which only covered some
contexts (glued `*N*eural`, `\\`+`*`, makecell). **Fix:** emit `#strong[...]` / `#emph[...]`
function form for body emphasis (robust), or insert a zero-width space at the unclean boundary
(same family as the `]`/`)`→`(` ZWSP fixes #239/#245). Highest ROI: unblocks 3 papers.

### 2. [CONTENT LOSS] `\def\input@path{...}` custom input search path not honored — pelegs
`gh-pelegs-maths-book` sets `\def\input@path{{./chapters/intro/}}` then `\input{preface}` etc.
ByeTex resolves `\input` only relative to the file/base dir, **not** the LaTeX `\input@path`, so
every chapter file fails to resolve and is **silently dropped** — 493 lines of output with **0
headings** (8 chapters in source). Only the cover/title front-matter leaked through. **Fix:** track
`\def\input@path` (and `\graphicspath`-style search lists) during the prepass and consult it in
`\input`/`\include` resolution; at minimum, *warn loudly* on an unresolved `\input` instead of
silently emitting nothing. Common pattern in structured books → high value.

### 3. [COMPILE BLOCKER] Counter `.display()` glued to following token — 2 papers
- `gh-sikatikenmogne-report:242` — `#context counter(heading.1).display()20pt` → "expected comma"
  (a dimension `20pt` from the source heading macro is glued straight onto the call).
- `ctan-memoir:5594` — `#context counter(heading.1).display() …` in a run-in context.

`\thechapter`/`\thesection`-style counters are emitted as `#context counter(...).display()` and then
butted against adjacent text/dimensions. **Fix:** emit a separator/ZWSP after `.display()` when the
next source char would form an invalid call-chain; also verify the `counter(heading.1)` target
syntax (a leaked `20pt` suggests an unconsumed dimension argument upstream).

### 4. [FIDELITY] `\tableofcontents` dropped everywhere — all 4 structured docs
Every passing book/thesis has `\tableofcontents` in source; **none** emit `#outline()`
(`#outline()=0` across all outputs). **Fix:** map `\tableofcontents`→`#outline()`,
`\listoffigures`→`#outline(target: figure.where(kind: image))`, etc. (currently `emit.rs` ~2781
silently drops them).

### 5. [FIDELITY] `\frontmatter` / `\mainmatter` / `\backmatter` dropped — amberj, kaobook
No page-numbering style switch (roman→arabic) and no front/main separation; currently dropped with a
warning (`emit.rs` ~2791). **Fix:** map to Typst page-numbering (`#set page(numbering: "i")` →
`"1"`) and the matching content flow.

### 6. [FIDELITY] Book layout compressed — amberj page_ratio 0.31 (5pp vs 16pp truth)
Content is faithful (word-recall 0.92, **heading-recall 1.00**) but the output is ~3× too short:
chapters don't start a new page, TOC/title-page spacing is absent, and book leading/margins aren't
matched. **Fix (separate density track):** chapter `#pagebreak(weak)`, a book/report spacing
profile. This is the prose-doc analogue of the existing arXiv density track.

### 7. [LEAKAGE / minor] preamble + `\setcounter` + orphan labels + encoding — pelegs (downstream of #2)
`\input{settings}` leaks tokens into the body (`\[table\]labelfont=…`, `\textproj\_`),
`\setcounter{chapter}{-1}` passes through raw, cross-refs to dropped content become
`#hide[#figure([]) <label>]`, and `François`→`Franois` (a `ç` is dropped — encoding/`\c{c}`).
Mostly a consequence of #2; revisit after #1–#3.

---

## Recommended fix order (Phase C/D input)
1. **#1 word-boundary emphasis** — unblocks 3 of 4 failures; bounded, well-understood bug class.
2. **#3 counter `.display()` glue** — unblocks the last 2 failures; same ZWSP family.
3. **#2 `\input@path`** — turns pelegs from empty→full body; protects all multi-file books.
4. **#4 `\tableofcontents`→`#outline()`** + **#5 frontmatter** — the universal structural fidelity wins.
5. **#6 book density** + **#7 leakage** — refinement, after the above.

Items #1–#3 are pure emitter bug-fixes (one TDD'd PR each, like the arXiv compile-blocker queue).
#4/#5 motivate the **DocType/DocProfile** refactor (Phase C): book/report/thesis should opt into
chapter levels + TOC + frontmatter, which the paper-centric profile never needed.

## Measurement infrastructure (findings, not bugs)
- **Ground-truth PDFs are the hard part for real-world projects.** Unlike arXiv (canonical PDF
  available), these need fonts / shell-escape / multi-pass that a hermetic single tectonic pass
  lacks. Only amberj produced a truth PDF → structural comparison is **partial** for this corpus;
  the **compile-in-typst** verdict is the robust signal. Future: a richer truth-build (font bundle,
  `--shell-escape`/inkscape, multi-pass) would widen structural coverage.
- **The sweep oracle conflates "truth-build failed" with "input broken".** It should distinguish
  *truth-unavailable (environment)* from *input genuinely malformed*, else real ByeTex output bugs
  hide as `INPUT_BROKEN` (exactly as warned in `project-compile-blockers-2026-06-13`). The 4
  failures here are ByeTex bugs verified by reading the emitted `.typ`.

## Reproduce
```bash
# Re-ingest (manifest records repo_ref / archive_sha256 for each):
python scripts/corpus_add_local.py --git https://github.com/<org>/<repo> --ref <SHA> \
  --id <gh-...> --source-kind github --doc-type <book|thesis|report> --title "..."
# Baseline:
BYETEX_BIN=target/release/byetex ./scripts/corpus_sweep.sh --with-oracle   # compile + attribution
uv run --with requests --with Pillow python scripts/visual_test.py --papers <id>  # structure (tectonic truth)
```
New non-arXiv ids are intentionally **out of `acceptance_baseline.json`** (measurement-only) until
the bugs above are triaged.
