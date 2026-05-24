---
name: byetex-using-warnings-json
description: How to read and act on a ByeTex warnings.json sidecar after a LaTeX -> Typst conversion. Read this skill BEFORE attempting to fix any warning emitted by `byetex convert`.
---

# Using ByeTex `warnings.json`

When you run `byetex convert input.tex`, ByeTex writes two files:

- `input.typ` — the converted Typst document.
- `input.warnings.json` — an array of structured warnings.

Each warning has this shape:

```json
{
  "range":      { "start_line": 42, "start_col": 1, "end_line": 47, "end_col": 18,
                  "byte_start": 1023, "byte_end": 1184 },
  "category":   { "kind": "tikz" },
  "severity":   "warning",
  "message":    "Human-readable explanation.",
  "snippet":    "exact source bytes that triggered this warning",
  "suggested_skill": "byetex-tikz-to-typst"
}
```

`severity` is one of `info`, `warning`, `error`.

## Workflow

1. Read `warnings.json`. If empty, the conversion was 100% clean — stop.
2. Group warnings by `category.kind`.
3. For each group, if `suggested_skill` is non-null, read that skill with
   `byetex skills read <name>` (or open `skills/<name>.md`) BEFORE editing the `.typ`.
4. Apply fixes to the `.typ` file at the line/column ranges given.
5. Re-run `typst compile input.typ` to confirm the document still builds.

## Common categories

| `category.kind`            | What it means                                       | Skill                              |
|----------------------------|-----------------------------------------------------|------------------------------------|
| `unsupported_command`      | Backslash command outside the v1 subset.            | (use general Typst knowledge)      |
| `unsupported_environment`  | LaTeX environment outside the v1 subset.            | `byetex-unsupported-environment`  |
| `custom_macro`             | User-defined `\newcommand`; body left as raw call.  | `byetex-custom-macros`            |
| `tikz`                     | TikZ picture; needs a CeTZ or sketch rewrite.       | `byetex-tikz-to-typst`            |
| `parse_error`              | tree-sitter could not parse this region.            | `byetex-parse-error`              |
| `ambiguous_math`           | Math command without a Typst equivalent.            | (use Typst math docs)              |
| `needs_manual_review`      | Construct converted approximately; verify manually. | (general)                          |
| `drop_only`                | Benign — already handled by ByeTex.                 | (no action needed)                 |

## Rules

- NEVER edit the `.tex` to "work around" a warning — fix the `.typ`.
- ALWAYS preserve the surrounding Typst structure (sections, labels).
- If `suggested_skill` is null, use general Typst knowledge and verify with `typst compile`.
