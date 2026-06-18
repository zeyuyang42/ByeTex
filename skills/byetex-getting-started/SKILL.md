---
name: byetex-getting-started
description: Cold-start overview for an agent about to repair a ByeTex conversion — which command to run first, the diagnose-first repair loop, and how to read skills. Read this before anything else.
---

# byetex: getting started

ByeTex deterministically converts an academic subset of LaTeX to Typst. For
anything it can't translate it emits a `warnings.json` sidecar and a catalogue of
**skills** that explain how to finish the job. Your task as an agent has two phases:
first get the generated `.typ` to a clean `typst compile`, then — if it already
compiles — raise its **fidelity** so the render matches the source (see "The fidelity
phase" below).

## Five commands

```bash
byetex convert paper.tex            # → paper.typ + paper.warnings.json + paper.agent_brief.md
byetex diagnose paper.tex           # → paper.typ + paper.diagnostics.json (compile errors mapped to LaTeX)
byetex diagnose paper.typ           # re-scan an ALREADY-EDITED .typ IN PLACE (no re-convert; edits preserved)
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

**Re-scanning after edits:** run `byetex diagnose paper.typ` (pass the **`.typ`**, not
the `.tex`) — it compiles the edited file IN PLACE and maps the typst errors WITHOUT
re-converting, so your edits survive (`src_fragment`/`skill_name` are null — there's
no source map for an edited file). Only `byetex diagnose paper.tex` (the **source**)
re-converts and overwrites `paper.typ`, so never run *that* between edits.

## The fidelity phase (when it already compiles)

A `diagnose` with an empty `diagnostics.json` means the `.typ` already compiles — now
the job is **fidelity**: make the render match the source. There are no compile errors
to map, so work from the warnings + a visual comparison:

1. Read `warnings.json` (`byetex skills read byetex-using-warnings-json`) — each entry
   names a construct ByeTex rendered approximately or dropped; fix the highest-impact
   ones via the `suggested_skill`.
2. Scan `paper.typ` for **leaked LaTeX** — raw `\command`, `_label`, `=10000`, or
   preamble tokens that rendered as body text — and delete/translate them.
3. Compare the rendered pages against the source PDF; patch the biggest visual gaps.
4. After a batch of edits, re-run `byetex diagnose paper.typ` to confirm you didn't
   introduce a compile error (it won't touch your edits).

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
