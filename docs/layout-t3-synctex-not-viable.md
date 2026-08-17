# T3 anchor-drift via SyncTeX — measured, not viable

**Decision: do not build it.** Plan step 8 proposed a fourth tier measuring where
a `\label`ed element *lands* — its page and position in the truth render versus in
our output — by reading SyncTeX data from `tectonic --synctex` alongside
`typst query` on our side. The Typst half works. The tectonic half does not carry
enough information, and building on it would produce a metric that is almost
entirely `unattributed`.

## What was measured

Tectonic 0.16.9, `--synctex`, on a corpus paper and on a minimal control.

### Corpus paper `2605.22776` (23 pages)

```
total lines      19031
Input: entries   67   empty filename: 59  (88%)
page records     23
box records      198
```

Two independent problems:

- **88% of `Input:` entries have an empty filename.** Box records address a file by
  index (`h1,46:` = file 1, line 46), so an unnamed index cannot be resolved back
  to a source file. Only index 1 is named, and it resolves to `main` — without the
  extension. This paper `\input`s many `.tex` files; labels in any of them are
  unaddressable.
- **198 box records for 23 pages** — about 8.6 per page. A usable SyncTeX carries
  thousands per page.

### Minimal control

```latex
\documentclass{article}
\begin{document}
\section{One}\label{sec:one}                          % line 3
Some prose here to occupy a paragraph.
\section{Two}\label{sec:two}                          % line 5
More prose in the second section.
\begin{equation}\label{eq:a} x = y \end{equation}     % line 7
\end{document}
```

```
MINIMAL 1-page doc: 90 lines | Input: 7 (empty 6) | boxes 1
lines referenced by boxes: [8]
source has labels on lines 3, 5, 7
```

**One box record, referencing line 8 — none of the three label lines.** So the
sparsity is not a property of a complicated paper; it is how this tectonic build
emits SyncTeX. `--keep-intermediates` changes nothing (still 1 box), and
`tectonic --help` offers no other SyncTeX option.

## Why this kills the design rather than degrading it

The plan already required T3 to be non-gating and to record
`anchor_drift_unattributed_frac`, anticipating that *some* boxes would be
unattributable. The measurement is not "some". At roughly one box per page and
88% of files unnamed, essentially every label is unattributable, and the tier
would emit a sidecar of `null` beside a fraction reading ~1.0.

That is the failure mode this project has hit three separate times already in
other forms — an empty measurement that reads as a clean one. Shipping it would
add a fourth thing for every loop tick to read, in exchange for no information.

## If someone wants T3 later

**Do not fix this by reading SyncTeX harder.** A content-anchored approach needs
no SyncTeX at all and reuses machinery that already exists:

1. resolve each `\label` to the text it names — a section title, a caption — from
   the LaTeX source; `collect_project_source` already walks the `\input` graph;
2. locate that text in the truth PDF with PyMuPDF `search_for()`;
3. locate the corresponding `<key>` anchor in our output via `typst query` and
   `location().position()` — already verified working;
4. compare the two positions.

That is the same content-anchored principle the rest of the harness rests on, and
it sidesteps the toolchain entirely. It is a *different design* from plan step 8,
not a repair of it, so it should be proposed on its own terms rather than
smuggled in under this step.

The other option — switching the truth render to a full TeX Live `pdflatex`,
which emits complete SyncTeX — changes the truth-render toolchain for every paper
and every existing metric. That is a much larger decision than one tier, and it
would invalidate the baseline that was just established.

## Reproducing

```bash
cd $(mktemp -d) && cat > t.tex <<'TEX'
\documentclass{article}
\begin{document}
\section{One}\label{sec:one}
Some prose here to occupy a paragraph.
\section{Two}\label{sec:two}
More prose in the second section.
\begin{equation}\label{eq:a} x = y \end{equation}
\end{document}
TEX
tectonic --synctex --outdir . t.tex
gunzip -c t.synctex.gz | grep -cE '^[hv][0-9]+,[0-9]+:'   # -> 1
```

## Status

Plan step 8: **closed as not viable**, with the evidence above. No code added.
Everything else in the layout tier — T0 anchors, T1 geometry, T2 ordered stream —
is built, measured, wired into the gate and into the loop.
