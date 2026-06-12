# AGENTS.md — start here

You are an AI agent about to repair a ByeTex conversion. ByeTex deterministically
converts LaTeX to [Typst](https://typst.app) — it works best on academic papers,
where its fidelity is tuned — and for anything it can't translate cleanly it emits
structured warnings and a catalogue of **skills**.
**Your job: take the generated `.typ` to a clean `typst compile`.**

This file is the orientation. The deeper technical reference is
[`docs/for-agents.md`](docs/for-agents.md); the per-construct fix guides are the
skills in [`skills/`](skills/) (also reachable as `byetex skills read <name>`).

## Five commands

```bash
byetex convert paper.tex      # → paper.typ + paper.warnings.json + paper.agent_brief.md
byetex diagnose paper.tex     # → paper.typ + paper.diagnostics.json (typst errors mapped to LaTeX)
byetex skills list            # list every repair skill
byetex skills read <name>     # read one skill's full body
typst compile paper.typ       # the success criterion
```

For a multi-file paper, pass the project directory (or the entry `.tex` with
`--project`): `byetex diagnose --project paper/main.tex`.

## The repair loop (diagnose-first)

1. **Diagnose once.** `byetex diagnose paper.tex` writes `paper.diagnostics.json`:
   an array of `{message, line, col, src_fragment, typ_region, skill_name}`, one
   per typst compile error.
2. **For each diagnostic:** read `src_fragment` (the LaTeX that produced the
   failing region). If `skill_name` is set, `byetex skills read <skill_name>`.
   Apply the **smallest** local edit to `paper.typ` that fixes that error.
3. **Verify:** `typst compile paper.typ`. Fix the next error, re-run, repeat.

### Critical rules

- **Do NOT re-run `byetex diagnose` between edits** — it re-converts from source
  and overwrites your edits to `paper.typ`. Iterate with `typst compile`; only
  re-run `diagnose` to start fresh from the LaTeX.
- **Edit the `.typ`, not the `.tex`.** You're fixing the conversion output.
- **Smallest local edit per error.** Don't rewrite whole blocks; preserve what
  already compiles.

## warnings.json vs diagnostics.json

| File | From | Contains |
|------|------|----------|
| `warnings.json` | `byetex convert` | Static conversion gaps ByeTex *knows* about (unsupported command, custom macro, tikz…), each with a `suggested_skill`. May still compile. |
| `diagnostics.json` | `byetex diagnose` | Actual `typst compile` errors, each mapped back to its LaTeX fragment + skill. These *block* compilation. |

Use `diagnose` when the goal is "make it compile"; consult `warnings.json` for
fidelity gaps that compile but render approximately. Start with
`byetex skills read byetex-getting-started`.

## Tiny worked example

```bash
$ byetex diagnose paper.tex
byetex diagnose: 1 typst error(s) → paper.diagnostics.json
# diagnostics.json: [{ "message": "unknown variable: foo",
#                      "src_fragment": "\\foo", "skill_name": "byetex-math", … }]
$ byetex skills read byetex-math        # → replace #text(red)[\foo] with the Typst symbol
# edit paper.typ …
$ typst compile paper.typ               # → paper.pdf, no errors. Done.
```

## When ByeTex isn't enough

If a region is hopeless (many `needs_manual_review` / `parse_error`), render the
original LaTeX fragment to PDF/SVG with `tectonic` or `pdflatex` and `#image(...)`
it from Typst. See [`skills/byetex-unsupported-environment.md`](skills/byetex-unsupported-environment.md).

---

For repository/development conventions (worktrees, TDD, the corpus gate), see
[`CLAUDE.md`](CLAUDE.md). For the converter's architecture, see
[`docs/architecture.md`](docs/architecture.md).
