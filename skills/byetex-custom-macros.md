---
name: byetex-custom-macros
description: Translate user-defined LaTeX `\newcommand` / `\def` macros into Typst functions. Use when a warning has `category.kind == "custom_macro"` or `unsupported_command` for a name not in the LaTeX standard.
---

# Translating LaTeX custom macros to Typst

LaTeX `\newcommand` defines a macro with optional argument count and a body.
Typst's equivalent is a function with `let name(args) = ...`. ByeTex leaves
custom macro **invocations** in the output as warnings; you translate both
the definition and each call site by hand.

## Identifying the definition

In the original `.tex`, find:

```latex
\newcommand{\foo}[2]{\textbf{#1}: #2}
```

This is a 2-argument macro that renders bold first arg + colon + second arg.

## Translating the definition

Place this near the top of the `.typ` (after imports, before content):

```typst
#let foo(arg1, arg2) = [*#arg1*: #arg2]
```

Rules:

- `[n]` → that many positional parameters in Typst.
- `#1`, `#2`, ... → `#arg1`, `#arg2`, ... in the function body.
- Wrap the body in `[ ... ]` if it produces content (most LaTeX macros do).
- Translate the LaTeX commands inside the body using the standard ByeTex
  mappings (`\textbf{X}` → `*#X*`, `\emph{X}` → `_#X_`, etc.).

## Translating the call sites

Each call `\foo{Hello}{world}` in the source becomes `#foo[Hello][world]` in
Typst. ByeTex marks these in the `.typ` as raw text passthrough; replace
them with the Typst-style call.

## When the macro uses TeX primitives

If `\newcommand` uses `\def`, `\expandafter`, `\csname`, or other low-level
TeX, the macro is likely too dynamic for direct translation. In that case:

1. Inline the macro at each call site (manually expand it once).
2. Document the inlining in a comment so future maintainers know.

## Verification

Run `typst compile <file>.typ` and verify each translated invocation renders
the same as the original LaTeX output.
