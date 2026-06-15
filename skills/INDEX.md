# ByeTex Skills Index

This directory holds skills (Claude Code plugin format: `<name>/SKILL.md`) that
AI agents read on demand to resolve warnings emitted by `byetex convert` and to
grade conversions. They are bundled in the ByeTex Claude Code plugin **and**
embedded into the `byetex` binary at build time.

**Cold start?** Read `byetex-getting-started` first — it explains which command
to run, the diagnose-first repair loop, and how to read these skills. Then
`byetex-using-warnings-json` for the warnings-sidecar shape. Each warning's
`suggested_skill` field names a skill in this directory.

## Skills

- `byetex-getting-started` — Cold-start overview: which command first, the
  repair loop, warnings.json vs diagnostics.json.
- `byetex-using-warnings-json` — How to read and act on the `warnings.json` sidecar.
- `byetex-repair-loop` — The `byetex diagnose` repair loop: iterating on compile
  errors without re-converting.
- `byetex-math` — Math gaps: `#text(red)[\foo]` placeholders, `op()`, `mat()`.
- `byetex-tikz-to-typst` — Migrating TikZ pictures to CeTZ.
- `byetex-custom-macros` — Translating user `\newcommand` / `\def`.
- `byetex-unsupported-environment` — Handling LaTeX envs outside v1.
- `byetex-parse-error` — Recovering regions tree-sitter could not parse.
- `byetex-bibliography` — `.bib` and `#bibliography(...)` handoff.
- `byetex-figures-subpar` — Figures + multi-caption `#subpar.grid` floats.
- `byetex-tables-layout` — Table fidelity + two-column / page-density notes.
- `byetex-visual-grading` — Grade visual fidelity of a conversion vs the LaTeX
  truth (truth↔typst page images) against `docs/fidelity-rubric.md`; emits
  structured findings. Build the packet with `byetex review <paper>`.

## Access

These skills are bundled three ways, so agents reach them without the source tree:

- **Claude Code plugin** — installed skills appear as `/byetex:<name>`.
- **CLI** — embedded into the `byetex` binary at build time:
  ```bash
  byetex skills list
  byetex skills read byetex-using-warnings-json
  ```
- **MCP** — when running `byetex serve`, call the `list_skills` / `read_skill` tools.
