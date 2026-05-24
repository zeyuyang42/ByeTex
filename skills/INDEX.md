# ByeTex Skills Index

This directory contains skills (Claude-Code-format markdown) that AI agents
read on demand to resolve warnings emitted by `byetex convert`.

Read `byetex-using-warnings-json.md` first — it documents the warning shape
and the standard workflow. Each warning's `suggested_skill` field names a
file in this directory.

## Skills

- `byetex-using-warnings-json.md` — Start here. How to read and act on the
  `warnings.json` sidecar.
- `byetex-tikz-to-typst.md` — Migrating TikZ pictures to CeTZ.
- `byetex-custom-macros.md` — Translating user `\newcommand` / `\def`.
- `byetex-unsupported-environment.md` — Handling LaTeX envs outside v1.
- `byetex-parse-error.md` — Recovering regions tree-sitter could not parse.
- `byetex-bibliography.md` — `.bib` and `#bibliography(...)` handoff.

## Programmatic access

These same skill files are embedded into the `byetex` binary at build time.
Agents can fetch them without needing the source tree:

```bash
byetex skills list
byetex skills read byetex-using-warnings-json
```

Or, when ByeTex is running in MCP server mode (`byetex serve`), call the
`list_skills` and `read_skill` MCP tools.
