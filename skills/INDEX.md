# ByeTex Skills Index

This directory contains skills (Claude-Code-format markdown) that AI agents
read on demand to resolve warnings emitted by `byetex convert`.

**Cold start?** Read `byetex-getting-started.md` first — it explains which
command to run, the diagnose-first repair loop, and how to read these skills.
Then `byetex-using-warnings-json.md` for the warnings-sidecar shape. Each
warning's `suggested_skill` field names a file in this directory.

## Skills

- `byetex-getting-started.md` — Cold-start overview: which command first, the
  repair loop, warnings.json vs diagnostics.json.
- `byetex-using-warnings-json.md` — How to read and act on the `warnings.json`
  sidecar.
- `byetex-repair-loop.md` — The `byetex diagnose` repair loop: iterating on
  compile errors without re-converting.
- `byetex-math.md` — Math gaps: `#text(red)[\foo]` placeholders, `op()`, `mat()`.
- `byetex-tikz-to-typst.md` — Migrating TikZ pictures to CeTZ.
- `byetex-custom-macros.md` — Translating user `\newcommand` / `\def`.
- `byetex-unsupported-environment.md` — Handling LaTeX envs outside v1.
- `byetex-parse-error.md` — Recovering regions tree-sitter could not parse.
- `byetex-bibliography.md` — `.bib` and `#bibliography(...)` handoff.
- `byetex-figures-subpar.md` — Figures + multi-caption `#subpar.grid` floats.
- `byetex-tables-layout.md` — Table fidelity + two-column / page-density notes.

## Programmatic access

These same skill files are embedded into the `byetex` binary at build time.
Agents can fetch them without needing the source tree:

```bash
byetex skills list
byetex skills read byetex-using-warnings-json
```

Or, when ByeTex is running in MCP server mode (`byetex serve`), call the
`list_skills` and `read_skill` MCP tools.
