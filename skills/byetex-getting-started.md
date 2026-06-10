---
name: byetex-getting-started
description: Cold-start overview for an agent about to repair a ByeTex conversion — which command to run first, the diagnose-first repair loop, and how to read skills. Read this before anything else.
---

# byetex: getting started

ByeTex deterministically converts an academic subset of LaTeX to Typst. For
anything it can't translate it emits a `warnings.json` sidecar and a catalogue of
**skills** that explain how to finish the job. Your task as an agent: take the
generated `.typ` to a clean `typst compile`.

## Five commands

```bash
byetex convert paper.tex            # → paper.typ + paper.warnings.json + paper.agent_brief.md
byetex diagnose paper.tex           # → paper.typ + paper.diagnostics.json (compile errors mapped to LaTeX)
byetex skills list                  # list every repair skill
byetex skills read <name>           # read one skill's full body
typst compile paper.typ             # the success criterion
```

Use `--project` (or pass a directory) for multi-file papers; `byetex diagnose
--project paper/main.tex` materialises a self-contained Typst project first.

## The repair loop (diagnose-first)

1. **Diagnose once.** `byetex diagnose paper.tex` writes `paper.diagnostics.json`:
   an array of `{message, line, col, src_fragment, typ_region, skill_name}`, one
   per typst compile error.
2. **For each diagnostic:** read `src_fragment` (the LaTeX that produced the
   failing region) and, if `skill_name` is set, `byetex skills read <skill_name>`.
   Apply the **smallest** local edit to `paper.typ`. Preserve what works.
3. **Verify:** `typst compile paper.typ`. Fix the next error and re-run until clean.

**Critical rule:** do NOT re-run `byetex diagnose` between edits — it re-converts
from source and overwrites your edits to `paper.typ`. Iterate with `typst
compile`; only re-run `diagnose` to start over from the LaTeX.

## warnings.json vs diagnostics.json

- **`warnings.json`** (from `convert`) — static conversion gaps ByeTex *knows*
  about (unsupported command, custom macro, tikz…). Each has a `suggested_skill`.
  Read `byetex-using-warnings-json`.
- **`diagnostics.json`** (from `diagnose`) — actual `typst compile` errors, each
  mapped back to its LaTeX fragment + skill. These are what *block compilation*.

Start with `diagnose` when the goal is "make it compile"; consult `warnings.json`
for fidelity gaps that compile but render approximately.

## When ByeTex isn't enough

If a region is hopeless (many `needs_manual_review` / `parse_error`), render the
original LaTeX fragment to PDF/SVG with `tectonic` or `pdflatex` and `#image(...)`
it from Typst. See `byetex-unsupported-environment`.
