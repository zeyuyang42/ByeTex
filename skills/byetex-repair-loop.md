---
name: byetex-repair-loop
description: The CLI repair loop — use `byetex diagnose` to compile the generated Typst, map each error to its LaTeX fragment + skill, fix the .typ, and verify with `typst compile`.
---

# byetex repair loop

When a converted paper doesn't compile, repair the `.typ` one error at a time.

## Loop

1. **Diagnose once.** `byetex diagnose paper.tex` writes:
   - `paper.typ` — the generated Typst.
   - `paper.diagnostics.json` — an array of `{message, line, col, src_fragment, typ_region, skill_name}`, one per typst error.
2. **For each diagnostic:**
   - Read `src_fragment` (the LaTeX that produced the failing region) and `typ_region`
     (the offending `.typ` line).
   - If `skill_name` is set, read it: `byetex skills read <skill_name>`.
   - Apply the **smallest** local edit to `paper.typ` that fixes that error. Preserve what
     already works.
3. **Verify.** Run `typst compile paper.typ`. If it still reports errors, fix the next one
   and re-run. Repeat until it compiles.

## Rules

- **Do NOT re-run `byetex diagnose` after editing** — it re-converts from source and
  overwrites your edits to `paper.typ`. Use `typst compile paper.typ` to iterate; only
  re-run `diagnose` to start over from the LaTeX source.
- `src_fragment` / `skill_name` are `null` when an error can't be mapped (e.g. it's in the
  preamble or a region you already edited) — fall back to the raw typst `message`.
- Fix the smallest, most local thing per error; don't rewrite whole blocks.
