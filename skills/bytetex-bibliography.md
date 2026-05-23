---
name: bytetex-bibliography
description: Handle the `.bib` bibliography after ByeTex converts a LaTeX document with `\bibliography{refs}`. Use when the output Typst has `#bibliography(...)` and the user needs to confirm the bib file resolves.
---

# Bibliography handoff

ByeTex translates LaTeX bibliography directives without parsing the `.bib`
file:

| LaTeX                                       | Typst                                       |
|---------------------------------------------|---------------------------------------------|
| `\bibliography{refs}`                       | `#bibliography("refs.bib")`                 |
| `\bibliographystyle{plain}` + `\bibliography{refs}` | `#bibliography("refs.bib", style: "plain")` |
| `\cite{einstein}`                           | `@einstein`                                 |
| `\cite{a,b,c}`                              | `@a @b @c`                                  |

## Workflow

1. Confirm the `.bib` file referenced by the `#bibliography(...)` call exists
   at the path. Typst resolves it relative to the `.typ` file.
2. Confirm the style argument is supported by Typst. Typst supports built-in
   styles like `"alphanumeric"`, `"author-date"`, `"chicago-author-date"`,
   `"ieee"`, `"mla"`, etc. If `\bibliographystyle{X}` used a custom `.bst`,
   you'll need a Typst CSL file or to pick the closest built-in.
3. Run `typst compile <file>.typ` and check for:
   - "label X does not exist": the citation key isn't in the `.bib`.
   - "bibliography file not found": the path needs fixing.

## Style mapping cheat sheet

| LaTeX style    | Closest Typst built-in       |
|----------------|------------------------------|
| `plain`        | `"alphanumeric"`             |
| `alpha`        | `"alphanumeric"`             |
| `abbrv`        | `"alphanumeric"`             |
| `unsrt`        | `"alphanumeric"`             |
| `apa`          | `"apa"`                      |
| `ieee`         | `"ieee"`                     |
| `chicago`      | `"chicago-author-date"`      |

If the project uses biblatex with `style=authoryear`, change to
`"chicago-author-date"` or `"author-date"`.

## Verification

`typst compile` should produce a PDF with the bibliography section auto-
populated. Each `@key` in the body should resolve to an entry.
