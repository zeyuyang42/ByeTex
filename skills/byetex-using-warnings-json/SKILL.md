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

1. Read `warnings.json`. If empty, all DROPPED constructs were handled — but that does
   **not** prove the body is leak-free: a *partially* translated construct (a `\begin{align}`
   / `\section` / `\begin{proof}` wrapper that leaked into the body as literal `\command`
   text) is NOT recorded in `warnings.json`. During fidelity work, also run the leak scan
   (see "Leaked LaTeX in the body" below).
2. Group warnings by `category.kind`.
3. For each group, if `suggested_skill` is non-null, read that skill with
   `byetex skills read <name>` (or open `skills/<name>.md`) BEFORE editing the `.typ`.
4. Apply the fix in the `.typ`. NOTE: `range` is the LaTeX **source** location, not a
   `.typ` line — find the spot in the `.typ` by searching for the rendered text /
   `snippet` (or run `byetex diagnose input.typ` for `.typ`-line-anchored compile errors).
5. Re-run `typst compile input.typ` to confirm the document still builds.

## Common categories

| `category.kind`            | What it means                                       | Skill / action                     |
|----------------------------|-----------------------------------------------------|------------------------------------|
| `unsupported_command`      | Backslash command outside the v1 subset.            | **Triage below** (don't guess)     |
| `unsupported_environment`  | LaTeX environment outside the v1 subset.            | `byetex-unsupported-environment`  |
| `custom_macro`             | User-defined `\newcommand`; body left as raw call.  | `byetex-custom-macros`            |
| `tikz`                     | TikZ picture; needs a CeTZ or sketch rewrite.       | `byetex-tikz-to-typst`            |
| `parse_error`              | tree-sitter could not parse this region.            | `byetex-parse-error`              |
| `ambiguous_math`           | Math command without a Typst equivalent.            | `byetex-math`                      |
| `needs_manual_review`      | Construct converted approximately; verify manually. | `byetex-unsupported-environment`  |
| `drop_only`                | Benign — already handled by ByeTex.                 | (no action needed)                 |

## Triaging `unsupported_command`

`unsupported_command` is a catch-all, so it points here rather than at one skill.
Look at the command name (`category.name` / `snippet`) and route it — **most are
benign** (a dropped no-output preamble command, not lost content):

| Command pattern                                                        | Action |
|------------------------------------------------------------------------|--------|
| `\STATE` `\State` `\FOR` `\ENDFOR` `\REQUIRE` `\Require` `\Ensure` …    | Algorithm pseudocode — `byetex-unsupported-environment` (its `algorithmic` body now renders as prose; reformat to a numbered block if you want the box). |
| `usepackage:<name>` (e.g. `latexsym`, `inconsolata`, `orcidlink`)      | Unsupported PACKAGE — **benign**, no body content lost. Ignore unless a specific symbol/font it provides renders wrong. |
| `\vskip` `\vspace` `\allowdisplaybreaks` `\begingroup` `\endgroup` `\onecolumn` `\twocolumn` `\clearpage` `\penalty` | Layout / spacing / grouping — **benign drop**, no content. Ignore. |
| `\DeclareMathAlphabet` `\SetMathAlphabet` `\newcolumntype` `\tcbset`   | Preamble config/declarations — **benign drop**. Ignore. |
| A `\command` that renders as literal text or a `"name"` string in the `.typ` | A math or custom command that wasn't translated — `byetex-math` (math context) or `byetex-custom-macros` (user `\newcommand`). |
| Anything else producing visible garbage in the body                    | Read `snippet`, decide math vs macro vs environment, and use the matching skill above. |

**Rule of thumb:** if the command had no visible output in the source (spacing,
counters, font declarations, package loads), the drop is correct — move on. Only act
when real content or a symbol is missing/wrong in the rendered `.typ`.

## Leaked LaTeX in the body (NOT in `warnings.json`)

`warnings.json` records constructs ByeTex **dropped**. It does **not** record a
*partial / garbled* translation — where a construct was converted but its wrapper
leaked into the body as literal text. Common shapes (all render as visible garbage,
none appear in `warnings.json`):

- a heading glued to its title: `\sectionIntroduction`, `\subsectionResults`
- an environment delimiter: `\begin{align}` / `\begin{proof}` / `\end{...}` sitting in the body
- a stray `\command` (`\smash`, `\hspace`, `\text`) or a `\[..\]` marker in running text

To find these, run the leak scan on the **`.typ`** (not the `.tex`):

```
byetex diagnose main.typ
```

Despite the "diagnose" name, on a `.typ` input this does BOTH a `typst` compile AND a
**leaked-LaTeX body scan** — it reports each residual `\command` / `\[..\]` fragment with
its `.typ` line. Fix each by translating or deleting the leaked fragment, then re-run it
to confirm the body is clean. (A clean `typst compile` does NOT imply a clean body — leaked
`\section` text compiles fine but renders wrong.)

## Rules

- NEVER edit the `.tex` to "work around" a warning — fix the `.typ`.
- ALWAYS preserve the surrounding Typst structure (sections, labels).
- If `suggested_skill` is null, use general Typst knowledge and verify with `typst compile`.
