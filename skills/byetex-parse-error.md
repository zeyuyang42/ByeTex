---
name: byetex-parse-error
description: Recover from a `parse_error` warning where tree-sitter could not parse a region of the LaTeX source. Use when a warning has `category.kind == "parse_error"`.
---

# Recovering from a parse error

ByeTex uses the tree-sitter-latex grammar, which is best-effort. Some inputs
defeat it — usually exotic TeX primitives, mismatched braces in custom macro
bodies, or deeply nested verbatim. The converter still emits something, but
the parse_error warning marks the region that could not be analyzed.

## What to do

1. Locate the byte range in the original `.tex` from the warning.
2. Look at the source around the parse_error region. Likely causes:
   - Mismatched `{` / `}` (very common).
   - `\verb!...!` with a non-`!` delimiter (the grammar mishandles some).
   - `\catcode` reassignments (out-of-scope for any structural parser).
   - Macro expansion that produces unbalanced LaTeX.
3. Decide a recovery action:
   - **Manual rewrite**: rewrite the offending region in Typst from scratch,
     ignoring what ByeTex emitted.
   - **Source repair**: if the LaTeX itself is malformed (e.g. missing `}`),
     fix the `.tex` and re-run `byetex convert`. This is usually the right
     fix when the LaTeX wouldn't compile either.
   - **Render-and-embed**: render the original LaTeX fragment to a PDF
     using `pdflatex` or `tectonic`, then `#image("fragment.pdf")` in Typst.

## Common patterns

- **Verbatim with unusual delimiters** (`\verb|...|`, `\verb"..."`): replace
  with Typst's raw block: ```` `code` ```` or triple-backtick fenced.
- **Custom command in math** that uses `\expandafter`: see
  `byetex-custom-macros` and inline the expanded form.
- **Comment with line-continuation**: a `%` followed by no newline at EOF can
  confuse the parser. Add a newline.

## Verification

After repair, run `byetex convert` again. The parse_error warning should
disappear. Then `typst compile <file>.typ`.
