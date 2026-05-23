# ByeTex Skills Index

This directory contains skills (Claude-Code-format markdown) that AI agents
read on demand to resolve warnings emitted by `bytetex convert`.

Read `bytetex-using-warnings-json.md` first — it documents the warning shape
and the standard workflow. Each warning's `suggested_skill` field names a
file in this directory.

## Skills

- `bytetex-using-warnings-json.md` — Start here. How to read and act on the
  `warnings.json` sidecar.
- `bytetex-tikz-to-typst.md` — Migrating TikZ pictures to CeTZ.
- `bytetex-custom-macros.md` — Translating user `\newcommand` / `\def`.
- `bytetex-unsupported-environment.md` — Handling LaTeX envs outside v1.
- `bytetex-parse-error.md` — Recovering regions tree-sitter could not parse.
- `bytetex-bibliography.md` — `.bib` and `#bibliography(...)` handoff.

## Programmatic access

These same skill files are embedded into the `bytetex` binary at build time.
Agents can fetch them without needing the source tree:

```bash
bytetex skills list
bytetex skills read bytetex-using-warnings-json
```

Or, when ByeTex is running in MCP server mode (`bytetex serve`), call the
`list_skills` and `read_skill` MCP tools.
