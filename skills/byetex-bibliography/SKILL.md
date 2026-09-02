---
name: byetex-bibliography
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
2. Confirm the style argument is one Typst accepts **for a bibliography**.
   Not every CSL style qualifies: `"alphanumeric"` and `"author-date"` are
   citation-only and make `typst compile` fail outright with
   *"CSL style … is not suitable for bibliographies"*. Verified working:
   `"ieee"`, `"apa"`, `"mla"`, `"chicago-author-date"`, `"springer-basic"`,
   `"nature"`, `"american-physics-society"`, `"council-of-science-editors"`.
   If `\bibliographystyle{X}` used a custom `.bst`, supply a Typst CSL file or
   pick the closest working built-in.
3. Run `typst compile <file>.typ` and check for:
   - "label X does not exist": the citation key isn't in the `.bib`.
   - "bibliography file not found": the path needs fixing.

## Style mapping cheat sheet

Every mapping below was checked by compiling `#bibliography(..., style: …)`
on Typst 0.14.

| LaTeX style    | Typst built-in          | why |
|----------------|-------------------------|-----|
| `plain`        | `"springer-basic"`      | numeric, alphabetical by author |
| `plainnat`     | `"springer-basic"`      | natbib's numeric default |
| `abbrv`        | `"springer-basic"`      | numeric, abbreviated names |
| `unsrt`        | `"ieee"`                | numeric in citation order |
| `alpha`        | `"springer-basic"`      | Typst has no bibliography-suitable alpha-label style |
| `apa`          | `"apa"`                 | |
| `ieee`         | `"ieee"`                | |
| `chicago`      | `"chicago-author-date"` | |

**Do not use `"alphanumeric"` or `"author-date"`.** Both are citation-only CSL
styles: Typst rejects them as a `#bibliography` style and the compile fails with
*"CSL style … is not suitable for bibliographies"*. An earlier version of this
table mapped `plain`/`alpha`/`abbrv`/`unsrt` onto `"alphanumeric"`, so following
it turned a clean compile into a broken one.

If the project uses biblatex with `style=authoryear`, use
`"chicago-author-date"`.

## Verification

`typst compile` should produce a PDF with the bibliography section auto-
populated. Each `@key` in the body should resolve to an entry.
